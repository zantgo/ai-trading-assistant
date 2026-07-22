<script lang="ts">
    // Sibling harness to OverlayEffectHarness.svelte that records the
    // **decoupled** `setVisible + updateData` call pattern introduced
    // for the LIQ HEATMAP primitive (mirrors the VolumeProfilePrimitive
    // wiring). The contract this harness pins down:
    //
    //  - `setVisible(visible)` is called on every toggle change.
    //  - `updateData(data)` is called on every data change.
    //  - The data survives a toggle flip: `updateData(null)` is NEVER
    //    called as a side-effect of `setVisible(false)`.
    //
    // A regression that re-introduces the old `updateData(visible ? data
    // : null)` pattern will cause `updateData(null)` to be recorded at
    // every toggle-off and break the test below.

    import { onMount } from 'svelte';

    interface OverlayCell {
        showOverlay: boolean;
        data: unknown;
    }

    const localCell = $state<OverlayCell>({
        showOverlay: false,
        data: null,
    });

    const shared = ((globalThis as any).__overlayVisibleCell ??= localCell) as OverlayCell;

    interface VisiblePrimitive {
        setVisible: (v: boolean) => void;
        updateData: (snapshot: unknown) => void;
    }

    let primitive: VisiblePrimitive | null = null;

    function recordVisible(v: boolean) {
        (globalThis as any).__overlayVisibleCalls.push(['setVisible', v]);
    }
    function recordUpdate(arg: unknown) {
        (globalThis as any).__overlayVisibleCalls.push(['updateData', arg]);
    }

    onMount(() => {
        primitive = {
            setVisible: recordVisible,
            updateData: recordUpdate,
        };
    });

    // The pattern under test: visibility and data are independent calls.
    // This is the new pattern for LiquidationHeatmapPrimitive and mirrors
    // the VolumeProfilePrimitive wiring.
    $effect(() => {
        const visible = shared.showOverlay;
        const data = shared.data;
        if (!primitive) return;
        primitive.setVisible(visible);
        primitive.updateData(data);
    });
</script>

<!-- No markup: this component exists purely to host the effect. -->
