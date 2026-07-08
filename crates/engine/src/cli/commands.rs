use super::CliConsole;
use super::{BOLD, RESET, CYAN, RED, GREEN, YELLOW, truncate, opt_decimal, opt_decimal_short, RecvError};

use std::sync::atomic::Ordering;

use crate::registry;

impl CliConsole {
    pub(crate) fn cmd_help(&self) {
        println!("{}AI Trading Assistant — CLI Commands{}", BOLD, RESET);
        println!();
        println!("{}Instance Management:{}", CYAN, RESET);
        println!("  {:<30} Create a new trading instance", "add <BASE> <QUOTE>");
        println!("  {:<30} Pause instance (keep positions)", "pause <ID>");
        println!("  {:<30} Stop instance (close all positions)", "stop <ID>");
        println!("  {:<30} Delete instance permanently", "delete <ID>");
        println!("  {:<30} List all instances", "list");
        println!();
        println!("{}View & Analysis:{}", CYAN, RESET);
        println!("  {:<30} Detailed instance view", "show <ID>");
        println!("  {:<30} Indicator summary", "show <ID> charts");
        println!("  {:<30} Instance metrics", "show <ID> dashboard");
        println!("  {:<30} Trade history", "show <ID> trades");
        println!("  {:<30} Real-time price stream (Ctrl+C to stop)", "watch <ID> [tf_secs]");
        println!("  {:<30} General dashboard overview", "dashboard");
        println!("  {:<30} System heartbeat", "status");
        println!();
        println!("{}Trading:{}", CYAN, RESET);
        println!("  {:<30} Manual position open", "manual open <ID> <LONG|SHORT>");
        println!("  {:<30} Manual position close", "manual close <ID>");
        println!("  {:<30} Show safety status", "safety <ID>");
        println!("  {:<30} Reset loss counter", "safety reset <ID>");
        println!();
        println!("{}Communication:{}", CYAN, RESET);
        println!("  {:<30} Chat with AI Director", "chat <ID> <message>");
        println!("  {:<30} View global configuration", "config");
        println!();
        println!("{}System:{}", CYAN, RESET);
        println!("  {:<30} Show this help", "help");
        println!("  {:<30} Graceful shutdown", "quit");
    }

    pub(crate) async fn cmd_add(&self, args: &[&str]) {
        if args.len() < 2 {
            println!("{}Usage: add <BASE> <QUOTE>{}\nExample: add BTC USDT", RED, RESET);
            return;
        }
        let base = args[0].to_uppercase();
        let quote = args[1].to_uppercase();

        // Session-first model: ensure a session is active before spawning pipelines.
        // In CLI mode we auto-initialize a default paper session on first `add`.
        // Hyperliquid perpetuals settle in USDC, so the default uses USDC.
        if !self.workspace.session.active.load(Ordering::Relaxed) {
            match self
                .workspace
                .init_session(
                    crate::workspace::TradingMode::Paper,
                    crate::workspace::Currency::USDC,
                    crate::workspace::ExchangeChoice::Hyperliquid,
                    10000.0,
                    None,
                )
                .await
            {
                Ok(()) => println!(
                    "{}🚪 Auto-initialized default session (Paper, 10000 USDC, Hyperliquid).{}",
                    CYAN, RESET
                ),
                Err(e) => {
                    println!("{}❌ Failed to initialize session: {}{}", RED, e, RESET);
                    return;
                }
            }
        }

        match registry::add_instance(
            &self.workspace,
            (base.clone(), quote.clone()),
            self.llm_client.clone(),
        ).await {
            Ok(inst) => {
                println!("{}✅ Instance created: {} ({} {}){}",
                    GREEN, inst.id, base, quote, RESET);
            }
            Err(e) => {
                println!("{}❌ Failed: {}{}", RED, e, RESET);
            }
        }
    }

    pub(crate) async fn cmd_pause(&self, args: &[&str]) {
        let id = self.resolve_id(args, "pause");
        if id.is_empty() { return; }
        match registry::pause_instance(&self.workspace, &id).await {
            Ok(()) => println!("{}⏸️  Instance {} paused{}", YELLOW, id, RESET),
            Err(e) => println!("{}❌ {}{}", RED, e, RESET),
        }
    }

    pub(crate) async fn cmd_stop(&self, args: &[&str]) {
        let id = self.resolve_id(args, "stop");
        if id.is_empty() { return; }
        match registry::stop_instance(&self.workspace, &id).await {
            Ok(()) => println!("{}🛑 Instance {} stopped{}", RED, id, RESET),
            Err(e) => println!("{}❌ {}{}", RED, e, RESET),
        }
    }

    pub(crate) async fn cmd_delete(&self, args: &[&str]) {
        let id = self.resolve_id(args, "delete");
        if id.is_empty() { return; }
        match registry::delete_instance(&self.workspace, &id).await {
            Ok(()) => println!("{}🗑️  Instance {} deleted{}", YELLOW, id, RESET),
            Err(e) => println!("{}❌ {}{}", RED, e, RESET),
        }
    }

    pub(crate) async fn cmd_list(&self) {
        let instances = registry::list_instances(&self.workspace).await;
        if instances.is_empty() {
            println!("{}No active instances. Use 'add <BASE> <QUOTE>' to create one.{}", YELLOW, RESET);
            return;
        }

        println!("{}═══ Instances ═══════════════════════════════════════{}", BOLD, RESET);
        println!("{:<16} {:<12} {:<12} {:<14} {:<14} {:<16}",
            "ID", "Pair", "Status", "Capital", "Equity", "Losses");
        println!("{}", "-".repeat(84));

        for inst in &instances {
            let status_color = match inst.status.as_str() {
                "running" => GREEN,
                "paused" => YELLOW,
                _ => RED,
            };
            println!("{:<16} {:<12} {}{:<12}{} {:<14.2} {:<14.2} {}{:<16}{}",
                truncate(&inst.id, 15),
                inst.pair,
                status_color, inst.status, RESET,
                inst.initial_capital,
                inst.current_equity,
                if inst.consecutive_losses >= 3 { RED } else { "" },
                inst.consecutive_losses,
                RESET,
            );
        }
    }

    pub(crate) async fn cmd_show(&self, args: &[&str]) {
        if args.is_empty() {
            println!("{}Usage: show <ID> [charts|dashboard|trades]{}", RED, RESET);
            return;
        }
        let id = args[0];
        let sub = args.get(1).map(|s| s.to_lowercase()).unwrap_or_default();

        let instances = self.workspace.instances.read().await;
        let inst = instances.values().find(|i| i.id == id || i.id.starts_with(id)).cloned();
        drop(instances);

        let inst = match inst {
            Some(i) => i,
            None => {
                println!("{}❌ Instance '{}' not found{}", RED, id, RESET);
                return;
            }
        };

        match sub.as_str() {
            "trades" | "history" => self.show_trades(&inst).await,
            "dashboard" | "metrics" => self.show_dashboard_detail(&inst).await,
            "charts" | "indicators" => self.show_charts(&inst).await,
            _ => self.show_instance_detail(&inst).await,
        }
    }

    pub(crate) async fn show_instance_detail(&self, inst: &crate::instance::Instance) {
        let status = inst.config_state.read().await.status.clone();
        let safety = inst.safety.caution_level.read().await.clone();
        let losses = inst.safety.consecutive_losses.load(Ordering::Relaxed);
        let initial = inst.trading.read().await.initial_capital;
        let equity = inst.trading.read().await.current_equity;

        println!("{}═══ Instance: {} ({}) ═══{}", BOLD, inst.pair_display(), inst.id, RESET);
        println!("  Status:     {:?}", status);
        println!("  Safety:     {:?} ({} consecutive losses)", safety, losses);
        println!("  Capital:    ${:.2} → ${:.2}", initial, equity);
        if initial > 0.0 {
            let roi = ((equity - initial) / initial) * 100.0;
            let roi_color = if roi >= 0.0 { GREEN } else { RED };
            println!("  ROI:        {}{:+.2}%{}", roi_color, roi, RESET);
        }
        println!("  API Key:    {}", if inst.api_key_valid.load(Ordering::Relaxed) { "✅ Configured" } else { "❌ Not set" });
        println!("  Intervals:  slow={}s normal={}s fast={}s",
            inst.config_state.read().await.intervals.slow_seconds,
            inst.config_state.read().await.intervals.normal_seconds,
            inst.config_state.read().await.intervals.fast_seconds,
        );
    }

    pub(crate) async fn show_charts(&self, inst: &crate::instance::Instance) {
        let snap = inst.micro.latest.read().await;
        match snap.as_ref() {
            Some(s) => {
                println!("{}═══ Chart Data: {} ═══{}", BOLD, inst.pair_display(), RESET);
                println!("  Price:     {}", s.mid_price);
                println!("  RSI(14):   {}", opt_decimal(&s.rsi_14()));
                println!("  MACD Line: {}", opt_decimal(&s.macd_line()));
                println!("  MACD Sig:  {}", opt_decimal(&s.macd_signal()));
                println!("  ADX(14):   {}", opt_decimal(&s.adx_14()));
                println!("  ATR(14):   {}", opt_decimal(&s.atr_14()));
                println!("  BBWP:      {}", opt_decimal(&s.bbwp()));
                println!("  Squeeze:   {}  Mom: {}",
                    if s.squeeze_on().unwrap_or(false) { "ON" } else { "OFF" },
                    opt_decimal(&s.squeeze_momentum()));
            }
            None => println!("{}No market data yet.{}", YELLOW, RESET),
        }
    }

    pub(crate) async fn show_dashboard_detail(&self, inst: &crate::instance::Instance) {
        let initial = inst.trading.read().await.initial_capital;
        let equity = inst.trading.read().await.current_equity;
        let roi = if initial > 0.0 { ((equity - initial) / initial) * 100.0 } else { 0.0 };

        println!("{}═══ Dashboard: {} ═══{}", BOLD, inst.pair_display(), RESET);
        println!("  Capital:    ${:.2}", initial);
        println!("  Equity:     ${:.2}", equity);
        println!("  ROI:        {}{:+.2}%{}", if roi >= 0.0 { GREEN } else { RED }, roi, RESET);

        // Fetch trade stats from DB
        let trades: Vec<(f64, String)> = sqlx::query_as(
            "SELECT realized_pnl, direction FROM trade_telemetry_history WHERE symbol = ?1 ORDER BY exit_timestamp DESC LIMIT 100"
        )
        .bind(inst.symbol())
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        if !trades.is_empty() {
            let wins = trades.iter().filter(|(pnl, _)| *pnl > 0.0).count();
            let total = trades.len();
            let total_pnl: f64 = trades.iter().map(|(pnl, _)| pnl).sum();
            let win_rate = wins as f64 / total as f64 * 100.0;

            println!("  Trades:     {}", total);
            println!("  Win Rate:   {}{:.1}%{} ({}W / {}L)",
                if win_rate >= 50.0 { GREEN } else { RED }, win_rate, RESET, wins, total - wins);
            println!("  Total PnL:  {}{:+.2}${}",
                if total_pnl >= 0.0 { GREEN } else { RED }, total_pnl, RESET);
        } else {
            println!("  No trades recorded yet.");
        }
    }

    pub(crate) async fn show_trades(&self, inst: &crate::instance::Instance) {
        let trades: Vec<(i64, String, f64, f64, f64, String)> = sqlx::query_as(
            "SELECT id, direction, entry_price, exit_price, realized_pnl, trigger_source \
             FROM trade_telemetry_history WHERE symbol = ?1 ORDER BY exit_timestamp DESC LIMIT 20"
        )
        .bind(inst.symbol())
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        if trades.is_empty() {
            println!("{}No trades recorded.{}", YELLOW, RESET);
            return;
        }

        println!("{}═══ Trade History: {} ═══{}", BOLD, inst.pair_display(), RESET);
        println!("{:<8} {:<6} {:<12} {:<12} {:<14} {:<12}", "ID", "Dir", "Entry", "Exit", "PnL", "Source");
        println!("{}", "-".repeat(64));
        for (id, dir, entry, exit, pnl, source) in &trades {
            let pnl_color = if *pnl >= 0.0 { GREEN } else { RED };
            println!("{:<8} {:<6} {:<12.2} {:<12.2} {}{:<13.2}{} {:<12}",
                id, dir, entry, exit, pnl_color, pnl, RESET, source);
        }
    }

    pub(crate) async fn cmd_dashboard(&self) {
        let instances = registry::list_instances(&self.workspace).await;
        println!("{}═══ General Dashboard ═══{}", BOLD, RESET);

        let mut total_initial: f64 = 0.0;
        let mut total_equity: f64 = 0.0;

        for inst in &instances {
            total_initial += inst.initial_capital;
            total_equity += inst.current_equity;
        }
        let total_pnl = total_equity - total_initial;

        let overall_roi = if total_initial > 0.0 { (total_pnl / total_initial) * 100.0 } else { 0.0 };

        println!("  Instances:     {}", instances.len());
        println!("  Max:           {}", self.workspace.max_instances().await);
        println!("  Total Capital: ${:.2}", total_initial);
        println!("  Total Equity:  {}{:.2}${}", if total_equity >= total_initial { GREEN } else { RED }, total_equity, RESET);
        println!("  PnL:           {}{:+.2}${}", if total_pnl >= 0.0 { GREEN } else { RED }, total_pnl, RESET);
        println!("  ROI:           {}{:+.2}%{}", if overall_roi >= 0.0 { GREEN } else { RED }, overall_roi, RESET);

        if !instances.is_empty() {
            println!("\n{}═══ Per-Instance ═══{}", BOLD, RESET);
            for inst in &instances {
                let roi = if inst.initial_capital > 0.0 {
                    ((inst.current_equity - inst.initial_capital) / inst.initial_capital) * 100.0
                } else { 0.0 };
                println!("  {:<12} {:<12} ROI:{}{:+.1}%{} Losses:{}",
                    inst.id[..inst.id.len().min(11)].to_string(),
                    inst.pair,
                    if roi >= 0.0 { GREEN } else { RED }, roi, RESET,
                    inst.consecutive_losses,
                );
            }
        }
    }

    pub(crate) async fn cmd_status(&self) {
        println!("{}═══ System Status ═══{}", BOLD, RESET);
        let active = self.workspace.session.active.load(Ordering::Relaxed);
        let mode = self.workspace.session.trading_mode.read().await.clone();

        println!("  Session: {}", if active { format!("{}Active{}", GREEN, RESET) } else { format!("{}Inactive{}", RED, RESET) });
        if let Some(m) = mode {
            println!("  Mode:    {}", m.as_str());
        }
        println!("  Instances: {}", self.workspace.instance_count().await);
        println!("  DB:        {}", if self.pool.is_closed() { "Disconnected" } else { "Connected" });
    }

    pub(crate) async fn cmd_config(&self, _args: &[&str]) {
        let config = self.workspace.config.read().await;
        println!("{}═══ Global Configuration ═══{}", BOLD, RESET);
        println!("  Max Instances:         {}", config.workspace.max_instances);
        println!("  Default Pair:          {}", config.workspace.default_pair);
        println!("  Consecutive Caution:   {}", config.safety.consecutive_loss_caution);
        println!("  Consecutive Dropout:   {}", config.safety.consecutive_loss_dropout);
        println!("  Dropout Duration:      {}h", config.safety.dropout_duration_hours);
        println!("  Drawdown Limit:        {}%", config.safety.capital_drawdown_pct);
        println!("  Intervals:             s={}s n={}s f={}s",
            config.intervals.slow_seconds,
            config.intervals.normal_seconds,
            config.intervals.fast_seconds);
        println!("  API Failover Retries:  {}", config.api_failover.max_retries_per_call);
        println!("  API Failover Delay:    {}s", config.api_failover.retry_delay_seconds);
    }

    pub(crate) async fn cmd_safety(&self, args: &[&str]) {
        if args.is_empty() {
            println!("{}Usage: safety <ID> [reset]{}", RED, RESET);
            return;
        }
        let id = args[0];
        let reset = args.get(1).copied() == Some("reset");

        let instances = self.workspace.instances.read().await;
        let inst = instances.values().find(|i| i.id == id || i.id.starts_with(id)).cloned();
        drop(instances);

        match inst {
            Some(inst) => {
                if reset {
                    inst.safety.reset_consecutive_losses().await;
                    println!("{}✅ Safety counter reset for instance {} ({}){}",
                        GREEN, inst.pair_display(), inst.id, RESET);
                } else {
                    let ctx = inst.safety.get_caution_context().await;
                    println!("{}Safety: {} {} {}",
                        BOLD, inst.pair_display(), ctx, RESET);
                }
            }
            None => println!("{}❌ Instance '{}' not found{}", RED, id, RESET),
        }
    }

    pub(crate) async fn cmd_manual(&self, args: &[&str]) {
        if args.len() < 2 {
            println!("{}Usage: manual open <ID> <LONG|SHORT>{}", RED, RESET);
            println!("{}       manual close <ID>{}", RED, RESET);
            return;
        }
        let action = args[0];
        let id = args[1];

        let instances = self.workspace.instances.read().await;
        let inst = instances.values().find(|i| i.id == id || i.id.starts_with(id)).cloned();
        drop(instances);

        match inst {
            Some(inst) => match action {
                "open" => {
                    let dir = args.get(2).unwrap_or(&"LONG").to_uppercase();
                    inst.safety.reset_consecutive_losses().await;
                    println!("{}✋ Manual OPEN: {} direction={} (safety counter reset){}",
                        YELLOW, inst.pair_display(), dir, RESET);
                }
                "close" => {
                    inst.safety.reset_consecutive_losses().await;
                    println!("{}✋ Manual CLOSE: {} (safety counter reset){}",
                        YELLOW, inst.pair_display(), RESET);
                }
                _ => println!("{}Unknown action: {}. Use 'open' or 'close'.{}", RED, action, RESET),
            },
            None => println!("{}❌ Instance '{}' not found{}", RED, id, RESET),
        }
    }

    pub(crate) async fn cmd_chat(&self, args: &[&str]) {
        if args.len() < 2 {
            println!("{}Usage: chat <ID> <message>{}", RED, RESET);
            return;
        }
        let id = args[0];
        let message = args[1..].join(" ");

        let instances = self.workspace.instances.read().await;
        let inst = instances.values().find(|i| i.id == id || i.id.starts_with(id)).cloned();
        drop(instances);

        let inst = match inst {
            Some(i) => i,
            None => {
                println!("{}❌ Instance '{}' not found{}", RED, id, RESET);
                return;
            }
        };

        println!("{}💬 Sending to {} AI Director...{}", CYAN, inst.pair_display(), RESET);

        let llm = &self.llm_client;
        let messages = vec![crate::llm::ChatMessage {
            role: "user".into(),
            content: message,
        }];

        match llm.chat(messages, Some(&inst.pair_key())).await {
            Ok(reply) => {
                println!("{}🤖 AI Director:{}", BOLD, RESET);
                println!("{}", reply);
            }
            Err(e) => {
                println!("{}❌ Chat failed: {}{}", RED, e, RESET);
            }
        }
    }

    /// Resolve an instance ID from args[0] or prompt the user.
    fn resolve_id(&self, args: &[&str], _action: &str) -> String {
        args.first().map(|s| s.to_string()).unwrap_or_default()
    }

    pub(crate) async fn cmd_watch(&self, args: &[&str]) {
        let id = self.resolve_id(args, "watch");
        if id.is_empty() {
            println!("{}Usage: watch <ID> [tf_secs]{}", RED, RESET);
            return;
        }
        let tf_secs: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(60);

        let instances = self.workspace.instances.read().await;
        let inst = instances.values().find(|i| i.id == id || i.id.starts_with(&id)).cloned();
        drop(instances);

        let inst = match inst {
            Some(i) => i,
            None => {
                println!("{}❌ Instance '{}' not found{}", RED, id, RESET);
                return;
            }
        };

        let mut rx = {
            if tf_secs == 180 {
                inst.active_pair.fast.broadcast_tx.subscribe()
            } else if tf_secs == 300 {
                inst.active_pair.slow.broadcast_tx.subscribe()
            } else if tf_secs == 900 {
                inst.active_pair.r#macro.broadcast_tx.subscribe()
            } else {
                inst.active_pair.micro.broadcast_tx.subscribe()
            }
        };

        println!("{}═══ Live Stream: {} ({}s) ═══{}", BOLD, inst.pair_display(), tf_secs, RESET);
        println!("Ctrl+C or 'q' to stop");
        println!("{:<14} {:<7} {:<12} {:<12} {:<7} {:<7}", "Price", "RSI", "MACD", "Sig", "ADX", "SQZ");
        println!("{}", "-".repeat(64));

        loop {
            tokio::select! {
                biased;
                _ = tokio::signal::ctrl_c() => {
                    println!("\n🛑 Stream stopped.");
                    break;
                }
                result = rx.recv() => {
                    match result {
                        Ok(snap) => {
                            let sqz = if snap.squeeze_on().unwrap_or(false) { "ON" } else { "off" };
                            print!("\r{:<14} {:<7} {:<12} {:<12} {:<7} {:<7}",
                                format!("{:.4}", snap.mid_price),
                                opt_decimal_short(&snap.rsi_14()),
                                opt_decimal_short(&snap.macd_line()),
                                opt_decimal_short(&snap.macd_signal()),
                                opt_decimal_short(&snap.adx_14()),
                                sqz,
                            );
                        }
                        Err(RecvError::Lagged(n)) => {
                            eprintln!("\n⚠️  Dropped {} snapshots, resuming...", n);
                        }
                        Err(RecvError::Closed) => {
                            println!("\n📡 Broadcast channel closed.");
                            break;
                        }
                    }
                }
            }
        }
        println!();
    }
}

