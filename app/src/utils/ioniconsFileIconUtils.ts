/**
 * Ionicons File Icon Utilities
 * Uses Ionicons for file type icons with comprehensive coverage and modern design
 * Based on https://ionic.io/ionicons
 */

import { h } from 'vue';
import { NIcon } from 'naive-ui';

// Import Ionicons from @vicons/ionicons5 (only icons that actually exist)
import {
  // File and folder icons
  Folder,
  FolderOpen,
  Document,
  DocumentText,
  CodeSlash,
  Image,
  Videocam,
  MusicalNotes,
  Archive,
  Settings,
  Terminal,
  GitBranch,
  Book,
  Server,
  BarChart,
  PieChart,
  ColorPalette,
  // Additional icons for special files
  Code,
  CodeWorking,
  MusicalNote,
  Images,
  DocumentAttach,
  DocumentLock,
  Documents,
  GitCommit,
  GitCompare,
  GitMerge,
  GitNetwork,
  GitPullRequest,
  QrCode,
  VideocamOff,
} from '@vicons/ionicons5';

// File type to icon mapping
export interface IoniconsFileIconMapping {
  icon: any;
  color?: string;
}

// Comprehensive file extension to icon mapping using Ionicons
const IONICONS_FILE_ICON_MAP: Record<string, IoniconsFileIconMapping> = {
  // Directories
  folder: { icon: Folder },
  'folder-open': { icon: FolderOpen },

  // Programming Languages
  js: { icon: CodeSlash, color: '#f7df1e' },
  jsx: { icon: CodeSlash, color: '#61dafb' },
  ts: { icon: CodeSlash, color: '#3178c6' },
  tsx: { icon: CodeSlash, color: '#61dafb' },
  py: { icon: CodeSlash, color: '#3776ab' },
  java: { icon: CodeSlash, color: '#ed8b00' },
  c: { icon: CodeSlash, color: '#a8b9cc' },
  cpp: { icon: CodeSlash, color: '#00599c' },
  cxx: { icon: CodeSlash, color: '#00599c' },
  cc: { icon: CodeSlash, color: '#00599c' },
  h: { icon: CodeSlash, color: '#a8b9cc' },
  hpp: { icon: CodeSlash, color: '#00599c' },
  hh: { icon: CodeSlash, color: '#00599c' },
  go: { icon: CodeSlash, color: '#00add8' },
  rs: { icon: CodeSlash, color: '#ce422b' },
  php: { icon: CodeSlash, color: '#777bb4' },
  rb: { icon: CodeSlash, color: '#cc342d' },
  swift: { icon: CodeSlash, color: '#fa7343' },
  kt: { icon: CodeSlash, color: '#7f52ff' },
  scala: { icon: CodeSlash, color: '#dc322f' },
  jl: { icon: CodeSlash, color: '#9558b2' }, // Julia

  // Web Technologies
  html: { icon: CodeSlash, color: '#e34f26' },
  htm: { icon: CodeSlash, color: '#e34f26' },
  css: { icon: CodeSlash, color: '#1572b6' },
  scss: { icon: CodeSlash, color: '#cf649a' },
  sass: { icon: CodeSlash, color: '#cf649a' },
  less: { icon: CodeSlash, color: '#1d365d' },
  vue: { icon: CodeSlash, color: '#4fc08d' },
  svelte: { icon: CodeSlash, color: '#ff3e00' },

  // Configuration Files
  json: { icon: CodeSlash, color: '#000000' },
  yaml: { icon: CodeSlash, color: '#cb171e' },
  yml: { icon: CodeSlash, color: '#cb171e' },
  toml: { icon: Settings, color: '#9c4221' },
  xml: { icon: CodeSlash, color: '#005f9f' },
  ini: { icon: Settings },
  cfg: { icon: Settings },
  conf: { icon: Settings },

  // Jupyter Notebooks
  ipynb: { icon: Book, color: '#f37626' },

  // Documentation
  md: { icon: DocumentText, color: '#083fa1' },
  markdown: { icon: DocumentText, color: '#083fa1' },
  txt: { icon: DocumentText },
  rtf: { icon: DocumentText },

  // Images
  png: { icon: Image },
  jpg: { icon: Image },
  jpeg: { icon: Image },
  gif: { icon: Image },
  bmp: { icon: Image },
  webp: { icon: Image },
  svg: { icon: Image },
  ico: { icon: Image },
  tiff: { icon: Image },
  tif: { icon: Image },

  // Videos
  mp4: { icon: Videocam },
  avi: { icon: Videocam },
  mov: { icon: Videocam },
  wmv: { icon: Videocam },
  flv: { icon: Videocam },
  webm: { icon: Videocam },
  mkv: { icon: Videocam },

  // Audio
  mp3: { icon: MusicalNotes },
  wav: { icon: MusicalNotes },
  flac: { icon: MusicalNotes },
  aac: { icon: MusicalNotes },
  ogg: { icon: MusicalNotes },
  m4a: { icon: MusicalNotes },

  // Archives
  zip: { icon: Archive },
  rar: { icon: Archive },
  '7z': { icon: Archive },
  tar: { icon: Archive },
  gz: { icon: Archive },
  bz2: { icon: Archive },

  // Documents
  pdf: { icon: Document, color: '#dc3545' },
  doc: { icon: DocumentText, color: '#2b579a' },
  docx: { icon: DocumentText, color: '#2b579a' },
  xls: { icon: BarChart, color: '#217346' },
  xlsx: { icon: BarChart, color: '#217346' },
  ppt: { icon: ColorPalette, color: '#d24726' },
  pptx: { icon: ColorPalette, color: '#d24726' },

  // Data Files
  csv: { icon: BarChart, color: '#1f6b3a' },
  sql: { icon: Server, color: '#336791' },
  db: { icon: Server },
  sqlite: { icon: Server },

  // Shell Scripts
  sh: { icon: Terminal },
  bash: { icon: Terminal },
  zsh: { icon: Terminal },
  fish: { icon: Terminal },
  ps1: { icon: Terminal },
  bat: { icon: Terminal },
  cmd: { icon: Terminal },

  // Git
  gitignore: { icon: GitBranch },
  gitattributes: { icon: GitBranch },
  gitmodules: { icon: GitBranch },
  gitkeep: { icon: GitBranch },

  // Default fallback
  default: { icon: Document },
};

// Special file names that should have specific icons
const IONICONS_SPECIAL_FILE_NAMES: Record<string, IoniconsFileIconMapping> = {
  'package.json': { icon: Code, color: '#f7df1e' },
  'package-lock.json': { icon: Code, color: '#f7df1e' },
  'yarn.lock': { icon: Code, color: '#2c8ebb' },
  'composer.json': { icon: Code, color: '#777bb4' },
  'cargo.toml': { icon: Code, color: '#ce422b' },
  'pom.xml': { icon: Code, color: '#ed8b00' },
  'build.gradle': { icon: Code, color: '#02303a' },
  dockerfile: { icon: Code, color: '#2496ed' },
  'docker-compose.yml': { icon: Code, color: '#2496ed' },
  makefile: { icon: CodeWorking },
  'cmakelists.txt': { icon: Code, color: '#064f8c' },
  readme: { icon: DocumentText, color: '#083fa1' },
  'readme.md': { icon: DocumentText, color: '#083fa1' },
  license: { icon: DocumentText },
  changelog: { icon: DocumentText },
  authors: { icon: DocumentText },
  contributors: { icon: DocumentText },
  copying: { icon: DocumentText },
  install: { icon: DocumentText },
  vagrantfile: { icon: Code, color: '#1563ff' },
  rakefile: { icon: Code, color: '#cc342d' },
  'webpack.config': { icon: Code, color: '#8dd6f9' },
  gulpfile: { icon: Code, color: '#cf4647' },
  gruntfile: { icon: Code, color: '#fba919' },
  'karma.conf': { icon: Code, color: '#0a9fd4' },
  'jest.config': { icon: Code, color: '#c21325' },
  tsconfig: { icon: Code, color: '#3178c6' },
  'babel.config': { icon: Code, color: '#f9dc3e' },
  gemfile: { icon: Code, color: '#cc342d' },
  podfile: { icon: Code, color: '#ee312a' },
  procfile: { icon: Terminal },
  'autogen.sh': { icon: Terminal },
  'configure.ac': { icon: Code },
  'makefile.am': { icon: Code },
};

/**
 * Get the appropriate Ionicons icon for a file based on its name and extension
 */
export function getIoniconsFileIcon(
  fileName: string,
  isDirectory: boolean = false
): IoniconsFileIconMapping {
  // Handle directories
  if (isDirectory) {
    return IONICONS_FILE_ICON_MAP['folder'];
  }

  // Check for special file names first
  const lowerFileName = fileName.toLowerCase();
  if (IONICONS_SPECIAL_FILE_NAMES[lowerFileName]) {
    return IONICONS_SPECIAL_FILE_NAMES[lowerFileName];
  }

  // Extract file extension
  const extension = fileName.split('.').pop()?.toLowerCase() || '';

  // Check if we have a mapping for this extension
  if (IONICONS_FILE_ICON_MAP[extension]) {
    return IONICONS_FILE_ICON_MAP[extension];
  }

  // Return default icon for unknown file types
  return IONICONS_FILE_ICON_MAP['default'];
}

/**
 * Create a Vue component for rendering Ionicons file icons
 */
export function createIoniconsFileIconComponent(fileName: string, isDirectory: boolean = false) {
  const iconMapping = getIoniconsFileIcon(fileName, isDirectory);

  return () =>
    h(
      NIcon,
      {
        style: {
          color: iconMapping.color || 'inherit',
          fontSize: '16px',
        },
      },
      {
        default: () =>
          h(iconMapping.icon, {
            style: {
              fontSize: '16px',
              width: '16px',
              height: '16px',
            },
          }),
      }
    );
}

/**
 * Get Ionicons file icon mapping for use in tree components
 */
export function getIoniconsFileIconMapping(
  fileName: string,
  isDirectory: boolean = false
): IoniconsFileIconMapping {
  return getIoniconsFileIcon(fileName, isDirectory);
}
