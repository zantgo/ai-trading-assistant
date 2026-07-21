<script lang="ts">
    // Test harness that reproduces the *exact* $effect pattern from
    // PriceChart.svelte: a primitive reference that's null on first
    // synchronous pass and assigned during onMount, plus an early-return
    // guard, plus reactive toggle and data props.
    //
    // The cell lives on `globalThis.__overlayCell` so the test file
    // (a plain `.ts`, which can't import `$state` runes) can mutate it
    // across mount/unmount cycles. We initialize it via a guard pattern
    // that keeps `$state` at the variable declaration site.

    import { onMount } from 'svelte';

    interface OverlayCell {
        showOverlay: boolean;
        data: unknown;
    }

    // Always run `$state(...)` at the declaration site (Svelte's
    // compiler requires this). On first mount this is the shared
    // cell. On subsequent mounts we discard this local and reuse
    // the shared cell from globalThis.
    const localCell = $state<OverlayCell>({
        showOverlay: false,
        data: null,
    });

    const shared = ((globalThis as any).__overlayCell ??= localCell) as OverlayCell;
    // If this is the first mount, `localCell` and `shared` are the same
    // reactive instance. If a previous mount already created the shared
    // cell, `shared` points to it and `localCell` is a throw-away.

    let primitive: { updateData: (snapshot: unknown) => void } | null = null;

    function record(arg: unknown) {
        (globalThis as any).__overlayCalls.push([arg]);
    }

    onMount(() => {
        primitive = { updateData: record };
    });

    // The pattern under test. The toggle flag and the data payload are
    // read BEFORE the early-return guard so Svelte 5 registers them as
    // dependencies of this effect. A regression that moves them after
    // the guard will break the test.
    $effect(() => {
        const visible = shared.showOverlay;
        const payload = shared.data;
        if (!primitive) return;
        primitive.updateData(visible ? payload : null);
    });
</script>

<!-- No markup: this component exists purely to host the effect. -->