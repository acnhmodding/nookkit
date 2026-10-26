use nookkit_layouts::kind::{Button, List};
use nookkit_layouts::{PageClass, PanePath, layouts, pages};

fn is_list(_: &PanePath<List>) {}
fn is_button(_: &PanePath<Button>) {}

#[test]
fn chara_make_page() {
    assert_eq!(pages::CharaMake::CLASS, "CharaMakePage");
    assert_eq!(pages::CharaMake::LAYOUT, "CharaMake");

    let eyes = pages::CharaMake::L_BTN_LIST_EYE_00;
    is_list(&eyes);
    assert_eq!(eyes.path(), "/L_BtnListEye_00");
    assert_eq!(eyes.layout(), "LBtnListCharaMakeEye");

    is_button(&pages::CharaMake::L_BTN_OK_00);
}

#[test]
fn nested_paths_join_with_slash() {
    let first = pages::CharaMake::L_BTN_LIST_EYE_00.then(layouts::LBtnListCharaMakeEye::L_BTN_00);
    is_button(&first);
    assert_eq!(first.path(), "/L_BtnListEye_00/L_Btn_00");
    assert_eq!(first.layout(), "LCharaMakeEyeBtn");
}

#[test]
fn class_differs_from_layout() {
    assert_eq!(pages::Address::CLASS, "AddressPage");
    assert_eq!(pages::Address::LAYOUT, "Addressee");
}
