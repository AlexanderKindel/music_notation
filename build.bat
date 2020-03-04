cl code_gen.c /Zi
code_gen.exe
cl music_notation.c /I content /I content/memory /I content/rational /I display /I display/viewport /I gui /I gui/clef_tab /I gui/staff_tab /I gui/staff_tab/edit_scales_dialog /Zi /D_DEBUG comctl32.lib gdi32.lib user32.lib