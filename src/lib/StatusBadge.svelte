<script lang="ts">
  export let label: string;
</script>

<span class="status-badge">{label}</span>

<style>
  .status-badge {
    display: inline-flex;
    min-width: 20px;
    height: 20px;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    padding: 0 6px;
    border: 1px solid currentColor;
    border-radius: 999px;
    color: var(--accent-strong);
    background: transparent;
    font-family: inherit;
    font-size: var(--type-caption);
    line-height: var(--line-caption);
    font-weight: 600;
    white-space: nowrap;
  }
</style>
