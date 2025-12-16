/**
 * Notebook Utilities
 * Helper functions for working with Jupyter notebooks
 */

/**
 * Creates an empty Julia Jupyter notebook structure
 * @returns JSON string for an empty Jupyter notebook with Julia kernel
 */
export function createEmptyJuliaNotebook(): string {
  const notebook = {
    nbformat: 4,
    nbformat_minor: 4,
    metadata: {
      kernelspec: {
        display_name: 'Julia 1.9+',
        language: 'julia',
        name: 'julia-1.9',
      },
      language_info: {
        name: 'julia',
        version: '1.9',
      },
    },
    cells: [],
  };

  return JSON.stringify(notebook, null, 2);
}
