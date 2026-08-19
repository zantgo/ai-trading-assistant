// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { cleanup, render, fireEvent, screen } from '@testing-library/svelte';
import LaunchSetup from './LaunchSetup.svelte';

beforeEach(() => {
    (globalThis as any).__appStore = {
        instancesMap: {},
        sessionCurrency: 'USDC',
        quote: 'USDC',
        pairKeyFor: (s: string) => (s.includes('-') ? s : `${s}-USDC`),
        initInstance: () => {},
        enterInstance: () => {},
        initSession: async () => ({ success: true }),
    };
});
afterEach(() => cleanup());

describe('debug2', () => {
    it('goToReview after adding instance', async () => {
        const { container } = await render(LaunchSetup);
        await fireEvent.click(screen.getByText('Continue'));
        console.log('STEP after 1:', (container.querySelector('h2') as HTMLElement)?.textContent);
        await fireEvent.click(screen.getByText('Continue'));
        console.log('STEP after 2:', (container.querySelector('h2') as HTMLElement)?.textContent);
        const baseInput = container.querySelector<HTMLInputElement>('#launch-base');
        await fireEvent.input(baseInput!, { target: { value: 'ETH' } });
        await fireEvent.click(screen.getByText('+ Add'));
        console.log('STEP after add:', (container.querySelector('h2') as HTMLElement)?.textContent);
        console.log('has Continue:', !!screen.queryByText('Continue'));
        await fireEvent.click(screen.getByText('Continue'));
        console.log('STEP after 3:', (container.querySelector('h2') as HTMLElement)?.textContent);
        expect(container.textContent).toContain('Review');
    });
});
