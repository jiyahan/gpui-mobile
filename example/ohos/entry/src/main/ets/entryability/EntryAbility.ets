import UIAbility from '@ohos.app.ability.UIAbility';
import window from '@ohos.window';
import native from 'libentry.so';
import { BACKGROUND_COLOR, BOTTOM_BAR_COLOR, FOREGROUND_COLOR } from '../theme/Theme';

export default class EntryAbility extends UIAbility {
  private mainWindow: window.Window | undefined;
  private readonly areaChanged = (): void => this.updateSafeArea();

  private updateSafeArea(): void {
    if (!this.mainWindow) {
      return;
    }
    let top = 0;
    let bottom = 0;
    let left = 0;
    let right = 0;
    const types: window.AvoidAreaType[] = [
      window.AvoidAreaType.TYPE_SYSTEM,
      window.AvoidAreaType.TYPE_CUTOUT,
      window.AvoidAreaType.TYPE_NAVIGATION_INDICATOR
    ];
    for (const type of types) {
      try {
        const area = this.mainWindow.getWindowAvoidArea(type);
        if (area.visible) {
          top = Math.max(top, area.topRect.height);
          bottom = Math.max(bottom, area.bottomRect.height);
          left = Math.max(left, area.leftRect.width);
          right = Math.max(right, area.rightRect.width);
        }
      } catch (error) {
        console.error(`Failed to read avoid area ${type}: ${error}`);
      }
    }
    native.setSafeArea(top, bottom, left, right);
  }

  onForeground(): void {
    native.setForeground(true);
    this.updateSafeArea();
  }

  onBackground(): void {
    native.setForeground(false);
  }

  onWindowStageCreate(windowStage: window.WindowStage): void {
    const mainWindow = windowStage.getMainWindowSync();
    this.mainWindow = mainWindow;
    mainWindow.on('avoidAreaChange', this.areaChanged);
    mainWindow.on('windowSizeChange', this.areaChanged);
    this.updateSafeArea();
    mainWindow.setWindowLayoutFullScreen(true).then(() => this.updateSafeArea()).catch((error: Error) => {
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
    native.setChromeCallback((statusColor: number | null, navigationColor: number | null,
                              lightContent: boolean): void => {
      const contentColor = lightContent ? '#FFFFFF' : '#000000';
      const properties: window.SystemBarProperties = {
        statusBarContentColor: contentColor,
        navigationBarContentColor: contentColor
      };
      if (statusColor !== null) {
        properties.statusBarColor = `#${statusColor.toString(16).padStart(6, '0')}`;
      }
      if (navigationColor !== null) {
        properties.navigationBarColor = `#${navigationColor.toString(16).padStart(6, '0')}`;
      }
      mainWindow.setWindowSystemBarProperties(properties).catch((error: Error) => {
        console.error(`Failed to apply system chrome: ${error.message}`);
      });
    });
    windowStage.loadContent('pages/Index');
  }

  onWindowStageDestroy(): void {
    native.setChromeCallback(null);
    if (this.mainWindow) {
      this.mainWindow.off('avoidAreaChange', this.areaChanged);
      this.mainWindow.off('windowSizeChange', this.areaChanged);
      this.mainWindow = undefined;
    }
  }
}
