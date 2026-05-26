/**
 * Svelte action: move the host element to document.body on mount,
 * remove it on destroy. Use for popovers / tooltips / context menus
 * that need to escape SVG / overflow:hidden ancestors.
 *
 * Usage:  <div use:portal>…</div>
 */
export function portal(node: HTMLElement) {
  document.body.appendChild(node);
  return {
    destroy() {
      if (node.parentNode === document.body) document.body.removeChild(node);
    },
  };
}
