pub mod errors;
pub mod file2dl;
pub mod metadata;
pub mod url;
#[tokio::test]
async fn case() {
    use file2dl::File2Dl;
    let mut files2dl = File2Dl::from("Downloads").unwrap();
    assert_eq!(
        files2dl
            .first()
            .unwrap()
            .size_on_disk
            .load(std::sync::atomic::Ordering::Relaxed),
        1001
    );
    for f in files2dl.iter_mut() {
        f.switch_status().unwrap();
        f.single_thread_dl().await.unwrap();
    }
    assert_eq!(
        files2dl
            .first()
            .unwrap()
            .size_on_disk
            .load(std::sync::atomic::Ordering::Relaxed),
        2048
    );
    // let mut file = File2Dl::new(
    //     "https://filesampleshub.com/download/document/txt/sample2.txt",
    //     "Downloads",
    // )
    // .await
    // .unwrap();
    // file.switch_status().unwrap();
    // file.single_thread_dl().await.unwrap();
    // assert_eq!(
    //     file.size_on_disk.load(std::sync::atomic::Ordering::Relaxed),
    //     2048
    // );
}
