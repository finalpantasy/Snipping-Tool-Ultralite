use snipping_tool_ul::export::save_image_to_path;
use std::{
    path::PathBuf,
    sync::{Arc, Barrier},
    time::{SystemTime, UNIX_EPOCH},
};

fn folder(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("ul-save-{name}-{}-{nonce}", std::process::id()));
    std::fs::create_dir(&path).unwrap();
    path
}

#[test]
fn saving_preserves_unrelated_part_file() {
    let dir = folder("sentinel");
    let path = dir.join("capture.png");
    let sentinel = dir.join("capture.png.part");
    std::fs::write(&sentinel, b"unrelated work").unwrap();
    save_image_to_path(vec![210, 30, 40, 255], 1, 1, &path).unwrap();
    assert_eq!(std::fs::read(&sentinel).unwrap(), b"unrelated work");
    assert_eq!(
        image::open(&path).unwrap().to_rgba8().into_raw(),
        [210, 30, 40, 255]
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn overlapping_saves_publish_whole_images_without_shared_staging() {
    let dir = folder("concurrent");
    let path = dir.join("capture.png");
    save_image_to_path([0, 55, 77, 255].repeat(256 * 256), 256, 256, &path).unwrap();
    let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let reader_done = done.clone();
    let reader_path = path.clone();
    let (ready, started) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut reads = 0;
        loop {
            let bytes = std::fs::read(&reader_path).unwrap();
            let image = image::load_from_memory(&bytes).unwrap().to_rgba8();
            assert_eq!(image.dimensions(), (256, 256));
            let first = *image.get_pixel(0, 0);
            assert!(image.pixels().all(|p| *p == first));
            assert!((0..6).any(|n| first.0 == [n * 30, 55, 77, 255]));
            reads += 1;
            if reads == 1 {
                ready.send(()).unwrap();
            }
            if reader_done.load(std::sync::atomic::Ordering::Acquire) {
                break;
            }
        }
        reads
    });
    started
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    let barrier = Arc::new(Barrier::new(6));
    let workers: Vec<_> = (0..6)
        .map(|n| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let pixels = [n as u8 * 30, 55, 77, 255].repeat(256 * 256);
                barrier.wait();
                save_image_to_path(pixels, 256, 256, &path)
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap().unwrap();
    }
    done.store(true, std::sync::atomic::Ordering::Release);
    assert!(reader.join().unwrap() >= 1);
    let image = image::open(&path).unwrap().to_rgba8();
    assert_eq!(image.dimensions(), (256, 256));
    let first = *image.get_pixel(0, 0);
    assert!(image.pixels().all(|pixel| *pixel == first));
    assert!((0..6).any(|n| first.0 == [n * 30, 55, 77, 255]));
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    std::fs::remove_dir_all(dir).unwrap();
}
