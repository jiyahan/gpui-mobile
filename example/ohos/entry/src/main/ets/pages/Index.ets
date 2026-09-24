import { BACKGROUND_COLOR, BOTTOM_BAR_COLOR } from '../theme/Theme';

@Entry
@Component
struct Index {
  @StorageProp('statusInsetPx') statusInsetPx: number = 0;

  build() {
    Column() {
      XComponent({ id: 'gpui-probe', type: 'surface', libraryname: 'entry' })
        .width('100%')
        .layoutWeight(1)
        .backgroundColor(BOTTOM_BAR_COLOR)
    }
    .width('100%')
    .height('100%')
    .padding({ top: this.getUIContext().px2vp(this.statusInsetPx) })
    .backgroundColor(BACKGROUND_COLOR)
  }
}
