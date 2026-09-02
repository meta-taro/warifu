<script lang="ts">
  import '$lib/styles/tokens.css';
  let { children } = $props();
</script>

<div class="app">
  {@render children()}
</div>

<style>
  :global(body) {
    margin: 0;
    background: var(--bg-app);
    color: var(--text-primary);
    font-family: var(--font-ui);
    font-size: var(--text-base-size);
    line-height: var(--text-base-line);
    -webkit-font-smoothing: antialiased;
  }
  .app {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
  }
</style>
