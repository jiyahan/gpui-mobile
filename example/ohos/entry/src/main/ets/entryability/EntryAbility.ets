import UIAbility from '@ohos.app.ability.UIAbility';
import window from '@ohos.window';
import native from 'libentry.so';
import { BACKGROUND_COLOR, BOTTOM_BAR_COLOR, FOREGROUND_COLOR } from '../theme/Theme';

export default class EntryAbility extends UIAbility {
  onForeground(): void {
    native.setForeground(true);
  }

  onBackground(): void {
    native.setForeground(false);
  }

  onWindowStageCreate(windowStage: window.WindowStage): void {
    const mainWindow = windowStage.getMainWindowSync();
    const statusInsetPx = mainWindow.getWindowAvoidArea(window.AvoidAreaType.TYPE_SYSTEM).topRect.height;
    AppStorage.setOrCreate('statusInsetPx', statusInsetPx);
    mainWindow.setWindowLayoutFullScreen(true).catch((error: Error) => {
      console.error(`Failed to enable immersive layout: ${error.message}`);
    });
    mainWindow.setPreferredOrientation(window.Orientation.AUTO_ROTATION).catch((error: Error) => {
      console.error(`Failed to enable auto rotation: ${error.message}`);
    });
    mainWindow.setWindowSystemBarProperties({
      statusBarColor: BACKGROUND_COLOR,
      statusBarContentColor: FOREGROUND_COLOR,
      navigationBarColor: BOTTOM_BAR_COLOR,
      navigationBarContentColor: FOREGROUND_COLOR
    }).catch((error: Error) => {
      console.error(`Failed to set system bar colors: ${error.message}`);
    });
    windowStage.loadContent('pages/Index');
  }
}
