<script lang="ts">
    import { onMount } from 'svelte';
    import { useAppStore } from '../state.svelte';
    import { generateIdenticonSvg } from '../lib/identicon';
    import Icon from '../lib/Icon.svelte';
    import styles from './UserProfile.module.css';

    const app = useAppStore();

    let draftName = $state('');
    let draftWallet = $state('');
    let draftApiKey = $state('');
    let draftApiSecret = $state('');
    let draftWalletSecret = $state('');
    let keyType = $state<'api' | 'wallet'>('wallet');
    let saveStatus = $state<'idle' | 'saving' | 'success' | 'error'>('idle');
    let saveError = $state('');

    const identiconSrc = $derived(generateIdenticonSvg(draftName || 'default', 96));

    onMount(async () => {
        const profile = await app.fetchProfile();
        if (profile) {
            draftName = profile.userName;
            draftWallet = profile.walletAddress;
        } else {
            draftName = app.sessionUserName || '';
            draftWallet = app.sessionWalletAddress || '';
        }
    });

    async function saveProfile() {
        saveStatus = 'saving';
        saveError = '';
        const ok = await app.saveProfile(draftName.trim(), draftWallet.trim());
        saveStatus = ok ? 'success' : 'error';
        if (!ok) saveError = 'Failed to save profile. Check server connection.';
        setTimeout(() => { if (saveStatus === 'success') saveStatus = 'idle'; }, 2000);
    }
</script>

<div class={styles.profilePage}>
    <div class={styles.profileCard}>
        <div class={styles.identiconSection}>
            <img src={identiconSrc} alt="Profile identicon" class={styles.identicon} width="96" height="96" />
            <div class={styles.identiconHint}>Generated from your name</div>
        </div>

        <div class={styles.fieldsSection}>
            <div class={styles.formGroup}>
                <label class={styles.formLabel} for="up-name">Display Name</label>
                <input
                    id="up-name"
                    type="text"
                    class={styles.formInput}
                    placeholder="e.g. Satoshi"
                    bind:value={draftName}
                    maxlength="30"
                />
            </div>

            <div class={styles.formGroup}>
                <span class={styles.formLabel}>Key Type</span>
                <div class={styles.radioGroup}>
                    <label class={styles.radioOption}>
                        <input
                            type="radio"
                            class={styles.radioInput}
                            name="keyType"
                            value="wallet"
                            bind:group={keyType}
                        />
                        <span class={styles.radioLabel}>Wallet Key</span>
                    </label>
                    <label class={styles.radioOption}>
                        <input
                            type="radio"
                            class={styles.radioInput}
                            name="keyType"
                            value="api"
                            bind:group={keyType}
                        />
                        <span class={styles.radioLabel}>API Key</span>
                    </label>
                </div>
            </div>

            {#if keyType === 'api'}
                <div class={styles.formGroup}>
                    <label class={styles.formLabel} for="up-api-key">API Key</label>
                    <input
                        id="up-api-key"
                        type="password"
                        class={styles.formInput}
                        placeholder="sk-..."
                        bind:value={draftApiKey}
                    />
                    <span class={styles.formHint}>Currently unavailable</span>
                </div>
                <div class={styles.formGroup}>
                    <label class={styles.formLabel} for="up-api-secret">Secret Key</label>
                    <input
                        id="up-api-secret"
                        type="password"
                        class={styles.formInput}
                        placeholder="••••••••"
                        bind:value={draftApiSecret}
                    />
                    <span class={styles.formHint}>Currently unavailable</span>
                </div>
            {:else}
                <div class={styles.formGroup}>
                    <label class={styles.formLabel} for="up-wallet">Wallet Address</label>
                    <input
                        id="up-wallet"
                        type="text"
                        class={styles.formInput}
                        placeholder="0x..."
                        bind:value={draftWallet}
                    />
                    <span class={styles.formHint}>Currently unavailable — saved for future integration</span>
                </div>
                <div class={styles.formGroup}>
                    <label class={styles.formLabel} for="up-wallet-secret">Secret Key</label>
                    <input
                        id="up-wallet-secret"
                        type="password"
                        class={styles.formInput}
                        placeholder="••••••••"
                        bind:value={draftWalletSecret}
                    />
                    <span class={styles.formHint}>Currently unavailable</span>
                </div>
            {/if}

            <button class={styles.saveBtn} onclick={saveProfile} disabled={saveStatus === 'saving'}>
                {saveStatus === 'saving' ? 'Saving...' : 'Save Profile'}
            </button>

            {#if saveStatus === 'success'}
                <div class={styles.successMsg}>Profile saved successfully.</div>
            {/if}
            {#if saveStatus === 'error' && saveError}
                <div class={styles.errorMsg}>{saveError}</div>
            {/if}

            <div class={styles.profileDivider}></div>

            <button class={styles.quitBtn} onclick={() => { app.showQuitDialog = true; }}>
                <Icon name="quit" size={14} /> Quit
            </button>
        </div>
    </div>
</div>
