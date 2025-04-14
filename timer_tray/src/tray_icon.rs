use image::GenericImageView;

use crate::GLOBAL_CANCEL;

pub(crate) struct TimerTray;

impl ksni::Tray for TimerTray {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            CheckmarkItem {
                label: "Quit".into(),
                checked: GLOBAL_CANCEL.is_cancelled(),
                activate: Box::new(|_| GLOBAL_CANCEL.cancel()),
                ..Default::default()
            }
            .into(),
        ]
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let img = image::load_from_memory_with_format(
            include_bytes!("icon.png"),
            image::ImageFormat::Png,
        )
        .expect("'icon.png' is not a valid file");

        let (width, height) = img.dimensions();
        let mut data = img.into_rgba8().into_vec();

        assert_eq!(data.len() % 4, 0);

        for pixel in data.chunks_exact_mut(4) {
            pixel.rotate_right(1) // rgba to argb
        }

        vec![ksni::Icon {
            width: width as i32,
            height: height as i32,
            data,
        }]
    }
}
