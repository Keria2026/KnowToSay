import { app } from './store.svelte';
import { api } from './api';
import { searchScripts } from '$lib/utils/scripts';
import { scopedScripts, scopedMaterials } from './search';
import type { MaterialSearchResult } from '$lib/types';

// Shared by the workbench and the independent search window. Empty quick
// searches browse replies; other scopes wait for a keyword before searching.
export class SearchResultsModel {
  limit = $state(50);
  hits = $state<MaterialSearchResult[]>([]);
  loading = $state(false);
  error = $state('');
  scripts = $derived.by(() => {
    const query = this.query().trim();
    return query || app.searchScope.area === 'quick'
      ? searchScripts(scopedScripts(app.nodes, app.searchScope), query)
      : [];
  });
  visibleScripts = $derived(this.scripts.slice(0, this.limit));
  materials = $derived(
    scopedMaterials(this.hits, app.searchScope, app.folders),
  );

  constructor(private query: () => string) {
    $effect(() => {
      const query = this.query().trim();
      const scope = app.searchScope;
      const workspace = app.workspace?.groupId;
      let active = true;
      this.limit = 50;
      this.hits = [];
      this.error = '';
      this.loading = false;
      if (!query || !workspace || !['all', 'library'].includes(scope.area))
        return;
      this.loading = true;
      const category =
        scope.area === 'library' && scope.group
          ? `&folder=${encodeURIComponent(scope.group)}`
          : '';
      const timer = setTimeout(() => {
        api<MaterialSearchResult[]>(
          `/search/materials?q=${encodeURIComponent(query)}${category}`,
        )
          .then((result) => {
            if (active) this.hits = result;
          })
          .catch((error) => {
            if (active) this.error = String(error);
          })
          .finally(() => {
            if (active) this.loading = false;
          });
      }, 120);
      return () => {
        active = false;
        clearTimeout(timer);
      };
    });
  }
}
