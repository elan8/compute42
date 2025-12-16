export interface FileTypeInfo {
  type: 'text' | 'image' | 'document' | 'binary' | 'csv' | 'notebook';
  language?: string;
  viewer: 'monaco' | 'image' | 'document' | 'binary' | 'csv' | 'notebook';
  extensions: string[];
  mimeTypes: string[];
}

export const FILE_TYPES: FileTypeInfo[] = [
  // Text files
  {
    type: 'text',
    language: 'julia',
    viewer: 'monaco',
    extensions: ['.jl'],
    mimeTypes: ['text/x-julia'],
  },
  {
    type: 'text',
    language: 'rust',
    viewer: 'monaco',
    extensions: ['.rs'],
    mimeTypes: ['text/x-rust'],
  },
  {
    type: 'text',
    language: 'javascript',
    viewer: 'monaco',
    extensions: ['.js', '.jsx'],
    mimeTypes: ['text/javascript', 'application/javascript'],
  },
  {
    type: 'text',
    language: 'typescript',
    viewer: 'monaco',
    extensions: ['.ts', '.tsx'],
    mimeTypes: ['text/typescript', 'application/typescript'],
  },
  {
    type: 'text',
    language: 'json',
    viewer: 'monaco',
    extensions: ['.json'],
    mimeTypes: ['application/json'],
  },
  {
    type: 'text',
    language: 'html',
    viewer: 'monaco',
    extensions: ['.html', '.htm'],
    mimeTypes: ['text/html'],
  },
  {
    type: 'text',
    language: 'css',
    viewer: 'monaco',
    extensions: ['.css'],
    mimeTypes: ['text/css'],
  },
  {
    type: 'text',
    language: 'python',
    viewer: 'monaco',
    extensions: ['.py'],
    mimeTypes: ['text/x-python'],
  },
  {
    type: 'text',
    language: 'markdown',
    viewer: 'monaco',
    extensions: ['.md', '.markdown'],
    mimeTypes: ['text/markdown'],
  },
  {
    type: 'text',
    language: 'yaml',
    viewer: 'monaco',
    extensions: ['.yml', '.yaml'],
    mimeTypes: ['text/yaml', 'application/x-yaml'],
  },
  {
    type: 'text',
    language: 'toml',
    viewer: 'monaco',
    extensions: ['.toml'],
    mimeTypes: ['text/x-toml'],
  },
  {
    type: 'text',
    language: 'shell',
    viewer: 'monaco',
    extensions: ['.sh', '.bash', '.zsh'],
    mimeTypes: ['text/x-sh', 'application/x-sh'],
  },
  {
    type: 'text',
    language: 'java',
    viewer: 'monaco',
    extensions: ['.java'],
    mimeTypes: ['text/x-java-source'],
  },
  {
    type: 'text',
    language: 'c',
    viewer: 'monaco',
    extensions: ['.c', '.h'],
    mimeTypes: ['text/x-c', 'text/x-csrc'],
  },
  {
    type: 'text',
    language: 'cpp',
    viewer: 'monaco',
    extensions: ['.cpp', '.cc', '.cxx', '.hpp', '.hh'],
    mimeTypes: ['text/x-c++src', 'text/x-c++hdr'],
  },
  {
    type: 'text',
    language: 'go',
    viewer: 'monaco',
    extensions: ['.go'],
    mimeTypes: ['text/x-go'],
  },
  {
    type: 'text',
    language: 'php',
    viewer: 'monaco',
    extensions: ['.php'],
    mimeTypes: ['text/x-php'],
  },
  {
    type: 'text',
    language: 'plaintext',
    viewer: 'monaco',
    extensions: ['.txt', '.log'],
    mimeTypes: ['text/plain'],
  },
  {
    type: 'text',
    language: 'plaintext',
    viewer: 'monaco',
    extensions: [], // No extension - for files like LICENSE, README, .gitignore, etc.
    mimeTypes: ['text/plain'],
  },

  // CSV files
  {
    type: 'csv',
    viewer: 'csv',
    extensions: ['.csv'],
    mimeTypes: ['text/csv', 'application/csv'],
  },

  // Jupyter Notebook files
  {
    type: 'notebook',
    viewer: 'notebook',
    extensions: ['.ipynb'],
    mimeTypes: ['application/x-ipynb+json', 'application/json'],
  },

  // Image files
  {
    type: 'image',
    viewer: 'image',
    extensions: ['.jpg', '.jpeg', '.png', '.gif', '.bmp', '.webp', '.svg', '.ico'],
    mimeTypes: [
      'image/jpeg',
      'image/png',
      'image/gif',
      'image/bmp',
      'image/webp',
      'image/svg+xml',
      'image/x-icon',
    ],
  },

  // Document files
  {
    type: 'document',
    viewer: 'document',
    extensions: ['.pdf'],
    mimeTypes: ['application/pdf'],
  },

  // Binary files (fallback)
  {
    type: 'binary',
    viewer: 'binary',
    extensions: [],
    mimeTypes: [],
  },
];

export function getFileTypeInfo(filePath: string): FileTypeInfo {
  const extension = filePath.toLowerCase().split('.').pop() || '';
  const fullExtension = extension ? `.${extension}` : '';
  const fileName =
    filePath
      .toLowerCase()
      .split(/[\/\\]/)
      .pop() || '';

  // First try to match by extension
  for (const fileType of FILE_TYPES) {
    if (fileType.extensions.includes(fullExtension)) {
      return fileType;
    }
  }

  // Check for common ASCII files by name (files without extensions or with dotfiles)
  const commonAsciiFiles = [
    'license',
    'readme',
    'changelog',
    'authors',
    'contributors',
    'copying',
    'install',
    'makefile',
    'dockerfile',
    'vagrantfile',
    'rakefile',
    '.gitignore',
    '.gitattributes',
    '.gitmodules',
    '.gitkeep',
    '.dockerignore',
    '.eslintignore',
    '.prettierignore',
    '.editorconfig',
    '.babelrc',
    '.eslintrc',
    '.prettierrc',
    'gemfile',
    'podfile',
    'procfile',
    'webpack.config',
    'gulpfile',
    'gruntfile',
    'karma.conf',
    'jest.config',
    'tsconfig',
    'babel.config',
    'package.json',
    'composer.json',
    'pom.xml',
    'build.xml',
    'cmakelists.txt',
    'makefile.am',
    'configure.ac',
    'autogen.sh',
  ];

  if (commonAsciiFiles.includes(fileName) || fileName.startsWith('.')) {
    // Return the plaintext type for common ASCII files
    return FILE_TYPES.find(
      (ft) => ft.type === 'text' && ft.language === 'plaintext' && ft.extensions.length === 0
    )!;
  }

  // If no match found, return binary as fallback
  return FILE_TYPES.find((ft) => ft.type === 'binary')!;
}

export function isImageFile(filePath: string): boolean {
  const fileType = getFileTypeInfo(filePath);
  return fileType.type === 'image';
}

export function isTextFile(filePath: string): boolean {
  const fileType = getFileTypeInfo(filePath);
  return fileType.type === 'text';
}

export function isDocumentFile(filePath: string): boolean {
  const fileType = getFileTypeInfo(filePath);
  return fileType.type === 'document';
}

export function isCsvFile(filePath: string): boolean {
  const fileType = getFileTypeInfo(filePath);
  return fileType.type === 'csv';
}

export function getLanguageFromPath(filePath: string): string {
  const fileType = getFileTypeInfo(filePath);
  return fileType.language || 'plaintext';
}

export function getViewerType(
  filePath: string
): 'monaco' | 'image' | 'document' | 'binary' | 'csv' | 'notebook' {
  const fileType = getFileTypeInfo(filePath);
  return fileType.viewer;
}
