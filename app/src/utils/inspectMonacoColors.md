# How to Debug Monaco Editor Colors

## Method 1: Browser Console (Easiest)

1. Open a Julia file in the Monaco editor
2. Open browser DevTools (F12)
3. Go to Console tab
4. Run: `debugMonacoColors()`

This will:

- Extract actual colors from the rendered Monaco editor
- Display them in the console
- Show a JSON object you can copy to update `juliaColorScheme.ts`

## Method 2: Manual Inspection

1. Open a Julia file in Monaco editor
2. Open DevTools (F12)
3. Use the element inspector to click on different syntax elements
4. In the Styles panel, check the computed `color` CSS property
5. Note the hex values for:
   - Keywords (function, if, end, etc.)
   - Functions (function names)
   - Variables
   - Strings
   - Comments
   - Types
   - Numbers

## Method 3: Programmatic Access

If you have access to the editor instance:

```typescript
import { debugMonacoColors } from '@/utils/debugMonacoColors';
const colors = debugMonacoColors(editorInstance);
```

## Updating the Color Scheme

After extracting colors, update `app/src/utils/juliaColorScheme.ts`:

```typescript
export const juliaColors: JuliaColorScheme = {
  keyword: '#EXTRACTED_COLOR', // Replace with actual color
  function: '#EXTRACTED_COLOR', // Replace with actual color
  // ... etc
};
```

Both Monaco and CodeMirror will automatically use the updated colors.
