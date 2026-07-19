<script lang="ts">
    import { getIcon } from './icons';

    interface Props {
        name: string;
        size?: 'sm' | 'md' | 'lg' | number;
    }

    let { name, size = 'md' }: Props = $props();
</script>

<span class="svg-icon">{@html getIcon(name, size)}</span>

<style>
    .svg-icon {
        display: inline-flex;
        align-items: center;
        justify-content: center;
    }
    :global(.svg-icon svg) {
        display: block;
    }
</style>
