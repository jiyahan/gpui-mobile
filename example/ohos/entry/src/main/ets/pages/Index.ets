@Entry
@Component
struct Index {
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
