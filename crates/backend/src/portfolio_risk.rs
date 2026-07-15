// Portfolio risk analytics.
//
// `validate_new_position` was a pre-trade manual-position validator backed
// by the removed `paper_trades` table. With manual trading gone, the
// validator is obsolete. `pearson_correlation` is preserved for the
// future automated-trading risk analytics (cross-pair correlation matrix).

pub fn pearson_correlation(x: &[f64], y: &[f64]) -> Option<f64> {
    let n = x.len().min(y.len());
    if n < 10 {
        return None;
    }
    let x_slice = &x[x.len() - n..];
    let y_slice = &y[y.len() - n..];
    let mean_x = x_slice.iter().sum::<f64>() / n as f64;
    let mean_y = y_slice.iter().sum::<f64>() / n as f64;
    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;
    for i in 0..n {
        let dx = x_slice[i] - mean_x;
        let dy = y_slice[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }
    if var_x == 0.0 || var_y == 0.0 {
        return None;
    }
    Some(cov / (var_x.sqrt() * var_y.sqrt()))
}
