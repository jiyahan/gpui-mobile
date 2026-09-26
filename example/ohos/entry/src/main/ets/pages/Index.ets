import native from 'libentry.so';

@Entry
@Component
struct Index {
  onBackPress(): boolean {
    return native.dispatchBack();
  }

  build() {
    Column() {
      XComponent({ id: 'gpui-probe', type: 'surface', libraryname: 'entry' })
        .width('100%')
        .layoutWeight(1)
    }
    .width('100%')
    .height('100%')
  }
}
