export const getBackgroundColor: () => number;
export const getForegroundColor: () => number;
export const getBottomBarColor: () => number;
export const setForeground: (foreground: boolean) => void;
export const dispatchBack: () => boolean;
export const setSafeArea: (top: number, bottom: number, left: number, right: number) => void;
export const setChromeCallback: (callback: ((statusColor: number | null, navigationColor: number | null, lightContent: boolean) => void) | null) => void;
