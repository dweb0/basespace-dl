// use basespace_dl::util::s3_etag;

// #[test]
// fn test_verify_s3_etag() {

//     // I have uploaded this file to amazon to get
//     // the expected etag
//     let part_size = 1024 * 1024 * 8;
//     let file_size = 1024 * 1024 * 26;
//     let artifical_file = vec![b'b'; file_size];
//     let expected_etag = "f6df70255a555bf8c077e5c95f97c1a0-2";

//     let actual_etag = s3_etag(artifical_file.as_slice(), file_size, part_size).unwrap();
//     assert_eq!(expected_etag, &actual_etag)
// }