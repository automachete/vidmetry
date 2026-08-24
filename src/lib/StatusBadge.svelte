<script lang="ts">
  export let label: string;
</script>

<span class="status-badge">{label}</span>

<style>
  .status-badge {
    display: inline-flex;
    min-height: 20px;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    padding: 1px 8px;
    border: 1px solid currentColor;
    border-radius: 999px;
    color: var(--accent-strong);
    background: transparent;
    font-family: inherit;
    font-size: var(--type-caption);
    line-height: var(--line-caption);
    font-weight: 650;
    letter-spacing: 0.01em;
    white-space: nowrap;
  }
</style>
