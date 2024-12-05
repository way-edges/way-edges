enum ClickedItem {
    TrayIcon,
    MenuItem(i32),
}

struct TrayHeadLayout {
    size: (i32, i32),
    // x, y
    icon_range: [[f64; 2]; 2],
}
impl TrayHeadLayout {
    fn get_clicked(&self, pos: (f64, f64)) -> bool {}
}

struct MenuRow {
    height_range: Vec<[f64; 2]>,
    id_vec: Vec<i32>,
}
impl MenuRow {
    fn get_clicked(&self, pos: (f64, f64)) -> Option<i32> {}
}

struct MenuLayout {
    size: (i32, i32),
    // x, y
    menu_col_width_range: Vec<[f64; 2]>,
    menu_row_of_each_col: Vec<MenuRow>,
}
impl MenuLayout {
    fn get_clicked(&self, pos: (f64, f64)) -> Option<i32> {}
}

struct TrayLayout {
    tray_head_layout: TrayHeadLayout,
    menu_layout: MenuLayout,
}
impl TrayLayout {
    fn get_clicked(&self, pos: (f64, f64)) -> Option<ClickedItem> {
        let max_size = (
            self.tray_head_layout.size.0.max(self.menu_layout.size.0) as f64,
            (self.tray_head_layout.size.1 + self.menu_layout.size.1) as f64,
        );
        if pos.1 < 0. || pos.0 < 0. || pos.0 > max_size.0 || pos.1 > max_size.1 {
            return None;
        };

        if pos.1 < self.tray_head_layout.size.1 as f64 {
            self.tray_head_layout
                .get_clicked(pos)
                .then_some(ClickedItem::TrayIcon)
        } else {
            self.menu_layout
                .get_clicked((pos.0, pos.1 - self.tray_head_layout.size.1 as f64))
                .map(ClickedItem::MenuItem)
        }
    }
}
