// Removed unused darkTheme import

// Define the primary color for the entire application
export const primaryColor: string = '#389826'; // Logo green
export const primaryColorHover: string = '#4AA830'; // Slightly lighter on hover
export const primaryColorPressed: string = '#2D7A1E'; // Darker when pressed
export const primaryColorSuppl: string = '#4AA830'; // Supplementary color

// Additional accent colors for different UI elements
export const accentColor: string = '#389826'; // Same as primary color
export const accentColorLight: string = '#4AA830'; // Same as primary hover
export const accentColorDark: string = '#2D7A1E'; // Same as primary pressed

// Export themeOverrides for use with n-config-provider
export const themeOverrides = {
  common: {
    primaryColor,
    primaryColorHover,
    primaryColorPressed,
    primaryColorSuppl,
    infoColor: accentColor,
    infoColorHover: accentColorLight,
    infoColorPressed: accentColorDark,
    successColor: accentColor,
    successColorHover: accentColorLight,
    successColorPressed: accentColorDark,
    warningColor: '#4AA830',
    warningColorHover: '#5BB940',
    warningColorPressed: '#3A8A20',
    errorColor: '#2D7A1E',
    errorColorHover: '#3A8A20',
    errorColorPressed: '#1F5A15',
  },
  Button: {
    colorPrimary: primaryColor,
    colorPrimaryHover: primaryColorHover,
    colorPrimaryPressed: primaryColorPressed,
    colorPrimarySuppl: primaryColorSuppl,
    colorInfo: accentColor,
    colorInfoHover: accentColorLight,
    colorInfoPressed: accentColorDark,
    colorSuccess: accentColor,
    colorSuccessHover: accentColorLight,
    colorSuccessPressed: accentColorDark,
    colorWarning: '#4AA830',
    colorWarningHover: '#5BB940',
    colorWarningPressed: '#3A8A20',
    colorError: '#2D7A1E',
    colorErrorHover: '#3A8A20',
    colorErrorPressed: '#1F5A15',
    textColorPrimary: '#fff',
    textColorHoverPrimary: '#fff',
    textColorPressedPrimary: '#fff',
    textColorFocusPrimary: '#fff',
    textColorDisabled: '#999999',
    border: `1px solid ${primaryColor}`,
    borderHover: `1px solid ${primaryColorHover}`,
    borderPressed: `1px solid ${primaryColorPressed}`,
    borderFocus: `1px solid ${primaryColor}`,
    borderDisabled: '1px solid #666666',
    rippleColor: primaryColor,
  },
};

// Utility functions for consistent color usage
export function getPrimaryColorWithOpacity(opacity: number = 0.1): string {
  const [r, g, b] = hexToRgb(primaryColor);
  return `rgba(${r}, ${g}, ${b}, ${opacity})`;
}

export function getAccentColorWithOpacity(opacity: number = 0.1): string {
  const [r, g, b] = hexToRgb(accentColor);
  return `rgba(${r}, ${g}, ${b}, ${opacity})`;
}

function hexToRgb(hex: string): [number, number, number] {
  hex = hex.replace('#', '');
  const r: number = parseInt(hex.substring(0, 2), 16);
  const g: number = parseInt(hex.substring(2, 4), 16);
  const b: number = parseInt(hex.substring(4, 6), 16);
  return [r, g, b];
}
