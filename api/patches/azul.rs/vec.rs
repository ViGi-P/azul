
    impl From<std::vec::Vec<u8>> for crate::vec::U8Vec {
        fn from(v: std::vec::Vec<u8>) -> crate::vec::U8Vec {
            crate::vec::U8Vec::copy_from(v.as_ptr(), v.len())
        }
    }

    impl From<crate::vec::U8Vec> for std::vec::Vec<u8> {
        fn from(v: crate::vec::U8Vec) -> std::vec::Vec<u8> {
            unsafe { std::slice::from_raw_parts(v.object.object.as_ptr(), v.object.object.len()).to_vec() }
        }
    }

    impl From<std::vec::Vec<std::string::String>> for crate::vec::StringVec {
        fn from(v: std::vec::Vec<std::string::String>) -> crate::vec::StringVec {
            let vec: Vec<AzString> = v.into_iter().map(|i| {
                let i: std::vec::Vec<u8> = i.into_bytes();
                az_string_from_utf8_unchecked(i.as_ptr(), i.len())
            }).collect();

            crate::vec::StringVec { object: az_string_vec_copy_from(vec.as_ptr(), vec.len()) }
        }
    }

    impl From<crate::vec::StringVec> for std::vec::Vec<std::string::String> {
        fn from(v: crate::vec::StringVec) -> std::vec::Vec<std::string::String> {
            v.leak().object
            .into_iter()
            .map(|s| unsafe {
                let s_vec: std::vec::Vec<u8> = s.into_bytes().into();
                std::string::String::from_utf8_unchecked(s_vec)
            })
            .collect()

            // delete() not necessary because StringVec is stack-allocated
        }
    }