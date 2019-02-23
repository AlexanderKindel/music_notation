#[repr(align(32))]
struct Template<A>
{
    data: A
}

static BLACK: COLORREF = 0;
static RED: COLORREF = 255;
static WHITE: COLORREF = 16777215;
static ADD_CLEF_DIALOG_TEMPLATE: Template<[u8; 766]> = Template{data: [1, 0, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 64, 8, 0, 0, 13, 0, 0, 0, 0, 0, 140, 0, 80, 0, 0, 0, 0, 0, 65, 0, 100, 0, 100, 0, 32, 0, 67, 0, 108, 0, 101, 0, 102, 0, 0, 0, 9, 0, 144, 1, 0, 1, 83, 0, 101, 0, 103, 0, 111, 0, 101, 0, 32, 0, 85, 0, 73, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 40, 0, 65, 0, 30, 0, 10, 0, 1, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 79, 0, 75, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 70, 0, 65, 0, 30, 0, 10, 0, 2, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 67, 0, 97, 0, 110, 0, 99, 0, 101, 0, 108, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 80, 5, 0, 5, 0, 40, 0, 10, 0, 0, 0, 0, 0, 115, 0, 116, 0, 97, 0, 116, 0, 105, 0, 99, 0, 0, 0, 67, 0, 108, 0, 101, 0, 102, 0, 32, 0, 115, 0, 104, 0, 97, 0, 112, 0, 101, 0, 58, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 80, 65, 0, 5, 0, 70, 0, 10, 0, 0, 0, 0, 0, 115, 0, 116, 0, 97, 0, 116, 0, 105, 0, 99, 0, 0, 0, 79, 0, 99, 0, 116, 0, 97, 0, 118, 0, 101, 0, 32, 0, 116, 0, 114, 0, 97, 0, 110, 0, 115, 0, 112, 0, 111, 0, 115, 0, 105, 0, 116, 0, 105, 0, 111, 0, 110, 0, 58, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 2, 16, 10, 0, 20, 0, 45, 0, 10, 0, 8, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 71, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 16, 10, 0, 30, 0, 45, 0, 10, 0, 9, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 67, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 16, 10, 0, 40, 0, 45, 0, 10, 0, 10, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 70, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 16, 10, 0, 50, 0, 45, 0, 10, 0, 11, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 85, 0, 110, 0, 112, 0, 105, 0, 116, 0, 99, 0, 104, 0, 101, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 2, 16, 70, 0, 15, 0, 30, 0, 10, 0, 12, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 49, 0, 53, 0, 109, 0, 97, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 16, 70, 0, 25, 0, 30, 0, 10, 0, 13, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 56, 0, 118, 0, 97, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 16, 70, 0, 35, 0, 30, 0, 10, 0, 14, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 78, 0, 111, 0, 110, 0, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 16, 70, 0, 45, 0, 30, 0, 10, 0, 15, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 56, 0, 118, 0, 98, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 16, 70, 0, 55, 0, 30, 0, 10, 0, 16, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 49, 0, 53, 0, 109, 0, 98, 0, 0, 0, 0, 0,]};
static ADD_STAFF_DIALOG_TEMPLATE: Template<[u8; 682]> = Template{data: [1, 0, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 64, 8, 0, 0, 10, 0, 0, 0, 0, 0, 165, 0, 80, 0, 0, 0, 0, 0, 65, 0, 100, 0, 100, 0, 32, 0, 83, 0, 116, 0, 97, 0, 102, 0, 102, 0, 0, 0, 9, 0, 144, 1, 0, 1, 83, 0, 101, 0, 103, 0, 111, 0, 101, 0, 32, 0, 85, 0, 73, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 80, 0, 65, 0, 30, 0, 10, 0, 2, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 67, 0, 97, 0, 110, 0, 99, 0, 101, 0, 108, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 50, 0, 65, 0, 30, 0, 10, 0, 1, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 79, 0, 75, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 80, 5, 0, 5, 0, 40, 0, 10, 0, 0, 0, 0, 0, 115, 0, 116, 0, 97, 0, 116, 0, 105, 0, 99, 0, 0, 0, 76, 0, 105, 0, 110, 0, 101, 0, 32, 0, 99, 0, 111, 0, 117, 0, 110, 0, 116, 0, 58, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 80, 45, 0, 5, 0, 20, 0, 10, 0, 9, 0, 0, 0, 115, 0, 116, 0, 97, 0, 116, 0, 105, 0, 99, 0, 0, 0, 53, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 22, 0, 0, 80, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 109, 0, 115, 0, 99, 0, 116, 0, 108, 0, 115, 0, 95, 0, 117, 0, 112, 0, 100, 0, 111, 0, 119, 0, 110, 0, 51, 0, 50, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 80, 5, 0, 25, 0, 60, 0, 10, 0, 0, 0, 0, 0, 115, 0, 116, 0, 97, 0, 116, 0, 105, 0, 99, 0, 0, 0, 83, 0, 99, 0, 97, 0, 108, 0, 101, 0, 58, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 85, 0, 25, 0, 75, 0, 10, 0, 11, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 65, 0, 100, 0, 100, 0, 32, 0, 110, 0, 101, 0, 119, 0, 32, 0, 115, 0, 99, 0, 97, 0, 108, 0, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 85, 0, 35, 0, 75, 0, 10, 0, 12, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 69, 0, 100, 0, 105, 0, 116, 0, 32, 0, 115, 0, 101, 0, 108, 0, 101, 0, 99, 0, 116, 0, 101, 0, 100, 0, 32, 0, 115, 0, 99, 0, 97, 0, 108, 0, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 85, 0, 45, 0, 75, 0, 10, 0, 13, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 82, 0, 101, 0, 109, 0, 111, 0, 118, 0, 101, 0, 32, 0, 115, 0, 101, 0, 108, 0, 101, 0, 99, 0, 116, 0, 101, 0, 100, 0, 32, 0, 115, 0, 99, 0, 97, 0, 108, 0, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 2, 0, 80, 5, 0, 35, 0, 70, 0, 100, 0, 10, 0, 0, 0, 67, 0, 79, 0, 77, 0, 66, 0, 79, 0, 66, 0, 79, 0, 88, 0, 0, 0, 0, 0, 0, 0,]};
static EDIT_STAFF_SCALE_DIALOG_TEMPLATE: Template<[u8; 378]> = Template{data: [1, 0, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 64, 8, 0, 0, 6, 0, 0, 0, 0, 0, 70, 0, 70, 0, 0, 0, 0, 0, 69, 0, 100, 0, 105, 0, 116, 0, 32, 0, 83, 0, 116, 0, 97, 0, 102, 0, 102, 0, 32, 0, 83, 0, 99, 0, 97, 0, 108, 0, 101, 0, 0, 0, 9, 0, 144, 1, 0, 1, 83, 0, 101, 0, 103, 0, 111, 0, 101, 0, 32, 0, 85, 0, 73, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 35, 0, 55, 0, 30, 0, 10, 0, 2, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 67, 0, 97, 0, 110, 0, 99, 0, 101, 0, 108, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 5, 0, 55, 0, 30, 0, 10, 0, 1, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 79, 0, 75, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 80, 5, 0, 5, 0, 60, 0, 10, 0, 0, 0, 0, 0, 115, 0, 116, 0, 97, 0, 116, 0, 105, 0, 99, 0, 0, 0, 78, 0, 97, 0, 109, 0, 101, 0, 58, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 80, 5, 0, 25, 0, 60, 0, 10, 0, 0, 0, 0, 0, 115, 0, 116, 0, 97, 0, 116, 0, 105, 0, 99, 0, 0, 0, 86, 0, 97, 0, 108, 0, 117, 0, 101, 0, 58, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 80, 5, 0, 15, 0, 60, 0, 10, 0, 8, 0, 0, 0, 101, 0, 100, 0, 105, 0, 116, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 80, 5, 0, 35, 0, 60, 0, 10, 0, 9, 0, 0, 0, 101, 0, 100, 0, 105, 0, 116, 0, 0, 0, 0, 0, 0, 0,]};
static REMAP_STAFF_SCALE_DIALOG_TEMPLATE: Template<[u8; 482]> = Template{data: [1, 0, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 64, 8, 0, 0, 4, 0, 0, 0, 0, 0, 125, 0, 85, 0, 0, 0, 0, 0, 82, 0, 101, 0, 109, 0, 97, 0, 112, 0, 32, 0, 83, 0, 116, 0, 97, 0, 102, 0, 102, 0, 32, 0, 83, 0, 99, 0, 97, 0, 108, 0, 101, 0, 0, 0, 9, 0, 144, 1, 0, 1, 83, 0, 101, 0, 103, 0, 111, 0, 101, 0, 32, 0, 85, 0, 73, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 60, 0, 70, 0, 30, 0, 10, 0, 2, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 67, 0, 97, 0, 110, 0, 99, 0, 101, 0, 108, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 30, 0, 70, 0, 30, 0, 10, 0, 1, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 79, 0, 75, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 80, 5, 0, 5, 0, 115, 0, 35, 0, 0, 0, 0, 0, 115, 0, 116, 0, 97, 0, 116, 0, 105, 0, 99, 0, 0, 0, 79, 0, 110, 0, 101, 0, 32, 0, 111, 0, 114, 0, 32, 0, 109, 0, 111, 0, 114, 0, 101, 0, 32, 0, 101, 0, 120, 0, 105, 0, 115, 0, 116, 0, 105, 0, 110, 0, 103, 0, 32, 0, 115, 0, 116, 0, 97, 0, 118, 0, 101, 0, 115, 0, 32, 0, 117, 0, 115, 0, 101, 0, 32, 0, 116, 0, 104, 0, 101, 0, 32, 0, 115, 0, 99, 0, 97, 0, 108, 0, 101, 0, 32, 0, 109, 0, 97, 0, 114, 0, 107, 0, 101, 0, 100, 0, 32, 0, 102, 0, 111, 0, 114, 0, 32, 0, 100, 0, 101, 0, 108, 0, 101, 0, 116, 0, 105, 0, 111, 0, 110, 0, 46, 0, 32, 0, 67, 0, 104, 0, 111, 0, 111, 0, 115, 0, 101, 0, 32, 0, 97, 0, 32, 0, 110, 0, 101, 0, 119, 0, 32, 0, 115, 0, 99, 0, 97, 0, 108, 0, 101, 0, 32, 0, 102, 0, 111, 0, 114, 0, 32, 0, 116, 0, 104, 0, 101, 0, 115, 0, 101, 0, 32, 0, 115, 0, 116, 0, 97, 0, 118, 0, 101, 0, 115, 0, 46, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 2, 0, 80, 5, 0, 40, 0, 110, 0, 100, 0, 8, 0, 0, 0, 67, 0, 79, 0, 77, 0, 66, 0, 79, 0, 66, 0, 79, 0, 88, 0, 0, 0, 0, 0, 0, 0,]};
static ADD_KEY_SIG_DIALOG_TEMPLATE: Template<[u8; 484]> = Template{data: [1, 0, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 64, 8, 0, 0, 7, 0, 0, 0, 0, 0, 70, 0, 75, 0, 0, 0, 0, 0, 65, 0, 100, 0, 100, 0, 32, 0, 75, 0, 101, 0, 121, 0, 32, 0, 83, 0, 105, 0, 103, 0, 110, 0, 97, 0, 116, 0, 117, 0, 114, 0, 101, 0, 0, 0, 9, 0, 144, 1, 0, 1, 83, 0, 101, 0, 103, 0, 111, 0, 101, 0, 32, 0, 85, 0, 73, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 5, 0, 60, 0, 30, 0, 10, 0, 2, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 67, 0, 97, 0, 110, 0, 99, 0, 101, 0, 108, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 35, 0, 60, 0, 30, 0, 10, 0, 1, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 79, 0, 75, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 80, 5, 0, 5, 0, 60, 0, 10, 0, 0, 0, 0, 0, 115, 0, 116, 0, 97, 0, 116, 0, 105, 0, 99, 0, 0, 0, 65, 0, 99, 0, 99, 0, 105, 0, 100, 0, 101, 0, 110, 0, 116, 0, 97, 0, 108, 0, 32, 0, 99, 0, 111, 0, 117, 0, 110, 0, 116, 0, 58, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 80, 25, 0, 15, 0, 20, 0, 10, 0, 0, 0, 0, 0, 115, 0, 116, 0, 97, 0, 116, 0, 105, 0, 99, 0, 0, 0, 49, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 22, 0, 0, 80, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 109, 0, 115, 0, 99, 0, 116, 0, 108, 0, 115, 0, 95, 0, 117, 0, 112, 0, 100, 0, 111, 0, 119, 0, 110, 0, 51, 0, 50, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 16, 5, 0, 35, 0, 45, 0, 10, 0, 9, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 83, 0, 104, 0, 97, 0, 114, 0, 112, 0, 115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 2, 16, 5, 0, 45, 0, 45, 0, 10, 0, 10, 0, 0, 0, 98, 0, 117, 0, 116, 0, 116, 0, 111, 0, 110, 0, 0, 0, 70, 0, 108, 0, 97, 0, 116, 0, 115, 0, 0, 0, 0, 0,]};
static BRAVURA_METADATA: FontMetadata = FontMetadata{black_notehead_stem_up_se: Point{x: 1.18, y: 0.168}, black_notehead_stem_down_nw: Point{x: 0.0, y: -0.168}, half_notehead_stem_up_se: Point{x: 1.18, y: 0.168}, half_notehead_stem_down_nw: Point{x: 0.0, y: -0.168}, beam_spacing: 0.25, beam_thickness: 0.5, double_whole_notehead_x_offset: 0.36, leger_line_thickness: 0.16, leger_line_extension: 0.4, staff_line_thickness: 0.13, stem_thickness: 0.12, };
