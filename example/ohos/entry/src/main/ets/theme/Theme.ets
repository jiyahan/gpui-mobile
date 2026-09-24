import native from 'libentry.so';

export const BACKGROUND_COLOR: string = `#${native.getBackgroundColor().toString(16).padStart(6, '0')}`;
export const FOREGROUND_COLOR: string = `#${native.getForegroundColor().toString(16).padStart(6, '0')}`;
export const BOTTOM_BAR_COLOR: string = `#${native.getBottomBarColor().toString(16).padStart(6, '0')}`;
