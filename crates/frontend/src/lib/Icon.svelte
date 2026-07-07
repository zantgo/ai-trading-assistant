<script lang="ts">
    import { iconHtml, type IconName } from './icons';

    let { name, size = 16 }: { name: IconName; size?: number } = $props();
    const inner = $derived(iconHtml(name));
</script>

<svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    width={size}
    height={size}
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="round"
    stroke-linejoin="round"
>
    {@html inner}
</svg>
