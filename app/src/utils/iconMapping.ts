// Icon mapping from Font Awesome to Vicons
// This file helps migrate from Font Awesome to Vicons for standardization

// Font Awesome to Vicons Ionicons5 mapping
export const faToIonicons5: Record<string, string> = {
  // File operations
  faFolderOpen: 'FolderOpenOutline',
  faFileAlt: 'DocumentOutline',
  faFolder: 'FolderOutline',
  faFileCode: 'CodeOutline',
  faEdit: 'CreateOutline',
  faCopy: 'CopyOutline',
  faTrash: 'TrashOutline',

  // Actions
  faPlay: 'PlayCircleOutline',
  faPlus: 'AddOutline',
  faFolderPlus: 'AddCircleOutline',
  faCog: 'SettingsOutline',
  faSpinner: 'RefreshOutline',
  faCheck: 'CheckmarkCircleOutline',
  faTimes: 'CloseCircleOutline',

  // UI elements
  faTerminal: 'TerminalOutline',
  faExternalLinkAlt: 'OpenOutline',
  faEnvelope: 'MailOutline',
  faChartBar: 'BarChartOutline',
  faChartLine: 'TrendingUpOutline',

  // Code/Development
  faCode: 'CodeOutline',
  faProjectDiagram: 'GitBranchOutline',
  faMicrochip: 'HardwareChipOutline',
  faCubes: 'CubeOutline',
  faFlaskVial: 'FlaskOutline',
  faHammer: 'HammerOutline',
};

// Font Awesome to Vicons Material mapping (for brand icons)
export const faToMaterial: Record<string, string> = {
  // Brand icons
  faJsSquare: 'JavascriptOutlined',
  faPython: 'CodeOutlined', // Python icon not available, using code icon
  faHtml5: 'HtmlOutlined',
  faCss3Alt: 'CssOutlined',
  faRust: 'CodeOutlined', // Rust icon not available, using code icon
  faVuejs: 'CodeOutlined', // Vue icon not available, using code icon
  faMarkdown: 'CodeOutlined', // Markdown icon not available, using code icon
  faGitAlt: 'GiteOutlined',
};

// Helper function to get the appropriate Vicons library for an icon
export function getViconsIcon(faIconName: string): {
  icon: string;
  library: 'ionicons5' | 'material';
} {
  if (faToMaterial[faIconName]) {
    return { icon: faToMaterial[faIconName], library: 'material' };
  }
  if (faToIonicons5[faIconName]) {
    return { icon: faToIonicons5[faIconName], library: 'ionicons5' };
  }
  // Default fallback
  return { icon: 'HelpOutline', library: 'ionicons5' };
}

// Common icon imports for easy migration
export const commonIonicons5 = [
  'FolderOpenOutline',
  'DocumentOutline',
  'FolderOutline',
  'CodeOutline',
  'CreateOutline',
  'CopyOutline',
  'TrashOutline',
  'PlayCircleOutline',
  'AddOutline',
  'AddCircleOutline',
  'SettingsOutline',
  'RefreshOutline',
  'CheckmarkCircleOutline',
  'CloseCircleOutline',
  'TerminalOutline',
  'OpenOutline',
  'MailOutline',
  'BarChartOutline',
  'TrendingUpOutline',
  'GitBranchOutline',
  'HardwareChipOutline',
  'CubeOutline',
  'FlaskOutline',
  'HammerOutline',
  'Save',
  'AlertCircleOutline',
  'Add',
  'Remove',
  'Resize',
  'InformationCircle',
  'Reload',
] as const;

export const commonMaterial = [
  'JavascriptOutlined',
  'CodeOutlined',
  'HtmlOutlined',
  'CssOutlined',
  'GiteOutlined',
] as const;
