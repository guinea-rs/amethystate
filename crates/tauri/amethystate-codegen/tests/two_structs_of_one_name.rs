use amethystate_codegen::{CodegenRegistry, Collision};

mod window {
    use amethystate::amethystate;

    #[amethystate(prefix = "window")]
    pub struct Config {
        #[amestate(default = 800u32)]
        pub width: u32,
    }
}

mod network {
    use amethystate::amethystate;

    #[amethystate(prefix = "network")]
    pub struct Config {
        #[amestate(default = String::new())]
        pub host: String,
    }
}

#[test]
fn two_structs_of_one_name_are_refused_rather_than_one_picked() {
    let refused = CodegenRegistry::new().err();

    assert_eq!(
        refused,
        Some(Collision {
            name: "Config",
            modules: [
                "two_structs_of_one_name::network",
                "two_structs_of_one_name::window"
            ],
        })
    );
    insta::assert_snapshot!(refused.unwrap().to_string());
}
