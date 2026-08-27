<script lang="ts">
    import { getIcon } from './icons';
    import styles from './SvgIcon.module.css';

    interface Props {
        name: string;
        size?: 'sm' | 'md' | 'lg' | number;
    }

    let { name, size = 'md' }: Props = $props();
</script>

<span class={styles.svgIcon}>{@html getIcon(name, size)}</span>
