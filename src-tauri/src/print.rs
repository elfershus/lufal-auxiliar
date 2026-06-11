use base64::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct PngLabel {
    pub png_b64: String,
}

#[tauri::command]
pub fn list_printers() -> Vec<String> {
    #[cfg(windows)]
    return imp::list_printers();
    #[cfg(not(windows))]
    return vec![];
}

#[tauri::command]
pub fn print_etiquetas(
    labels: Vec<PngLabel>,
    height_mm: f32,
    printer_name: String,
    paper_width_mm: f32,
) -> Result<(), String> {
    if labels.is_empty() {
        return Ok(());
    }
    #[cfg(windows)]
    return imp::print_labels(labels, height_mm, printer_name, paper_width_mm);
    #[cfg(not(windows))]
    return Err("Impresión directa solo disponible en Windows".to_string());
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::mem;
    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::{HANDLE, HWND},
            Graphics::{
                Gdi::{
                    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateDCW, DeleteDC,
                    DEVMODE_FIELD_FLAGS, DEVMODEW, DIB_RGB_COLORS, GetDeviceCaps,
                    HDC, HORZRES, RGBQUAD, SRCCOPY, StretchDIBits, VERTRES,
                },
                Printing::{
                    ClosePrinter, DocumentPropertiesW, EnumPrintersW,
                    OpenPrinterW, PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL,
                    PRINTER_INFO_2W,
                },
            },
            Storage::Xps::{DOCINFOW, EndDoc, EndPage, StartDocW, StartPage},
        },
    };

    const DM_PAPERSIZE:   DEVMODE_FIELD_FLAGS = DEVMODE_FIELD_FLAGS(0x0000_0002);
    const DM_PAPERLENGTH: DEVMODE_FIELD_FLAGS = DEVMODE_FIELD_FLAGS(0x0000_0004);
    const DM_PAPERWIDTH:  DEVMODE_FIELD_FLAGS = DEVMODE_FIELD_FLAGS(0x0000_0008);
    const DMPAPER_USER:   i16 = 256;

    pub fn list_printers() -> Vec<String> {
        unsafe {
            let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
            let mut needed: u32 = 0;
            let mut count: u32 = 0;

            // Primera llamada: tamaño de buffer necesario
            let _ = EnumPrintersW(flags, PCWSTR::null(), 2, None, &mut needed, &mut count);
            if needed == 0 {
                return vec![];
            }

            let mut buf = vec![0u8; needed as usize];
            if EnumPrintersW(
                flags,
                PCWSTR::null(),
                2,
                Some(&mut buf),
                &mut needed,
                &mut count,
            )
            .is_err()
                || count == 0
            {
                return vec![];
            }

            std::slice::from_raw_parts(buf.as_ptr() as *const PRINTER_INFO_2W, count as usize)
                .iter()
                .filter_map(|p| p.pPrinterName.to_string().ok())
                .collect()
        }
    }

    pub fn print_labels(
        labels: Vec<PngLabel>,
        height_mm: f32,
        printer_name: String,
        paper_width_mm: f32,
    ) -> Result<(), String> {
        // Decodificar todo antes de abrir el DC (fail-fast)
        let pngs: Vec<Vec<u8>> = labels
            .iter()
            .enumerate()
            .map(|(i, l)| {
                BASE64_STANDARD
                    .decode(&l.png_b64)
                    .map_err(|e| format!("base64 etiqueta {i}: {e}"))
            })
            .collect::<Result<_, _>>()?;

        let printer_w: Vec<u16> = printer_name.encode_utf16().chain(Some(0)).collect();
        let pcw = PCWSTR::from_raw(printer_w.as_ptr());

        // Dimensiones en décimas de mm: ancho configurable y longitud de la etiqueta
        let paper_w = (paper_width_mm * 10.0).round() as i16;
        let paper_h = (height_mm * 10.0).round() as i16;

        let dm_buf = build_devmode(pcw, paper_w, paper_h)?;

        let hdc = unsafe {
            CreateDCW(
                PCWSTR::null(),
                pcw,
                PCWSTR::null(),
                Some(dm_buf.as_ptr() as *const DEVMODEW),
            )
        };
        if hdc == HDC(std::ptr::null_mut()) {
            return Err("CreateDCW falló — impresora no disponible".to_string());
        }

        let result = do_print(hdc, &pngs);
        unsafe { let _ = DeleteDC(hdc); };
        result
    }

    // Obtiene el DEVMODE del driver y le aplica el tamaño de papel personalizado.
    fn build_devmode(pcw: PCWSTR, paper_w: i16, paper_h: i16) -> Result<Vec<u8>, String> {
        unsafe {
            let mut hprinter = HANDLE::default();
            OpenPrinterW(pcw, &mut hprinter, None)
                .map_err(|e| format!("OpenPrinterW: {e}"))?;

            // fmode = 0 → devuelve el tamaño requerido del buffer
            let needed =
                DocumentPropertiesW(HWND::default(), hprinter, pcw, None, None, 0);
            if needed < 0 {
                let _ = ClosePrinter(hprinter);
                return Err("DocumentPropertiesW: no se pudo obtener tamaño".to_string());
            }

            let mut buf = vec![0u8; needed as usize];
            // DM_OUT_BUFFER = 2
            let rc = DocumentPropertiesW(
                HWND::default(),
                hprinter,
                pcw,
                Some(buf.as_mut_ptr() as *mut DEVMODEW),
                None,
                2,
            );
            let _ = ClosePrinter(hprinter);

            if rc != 1 {
                return Err(format!("DocumentPropertiesW: error {rc}"));
            }

            let dm = &mut *(buf.as_mut_ptr() as *mut DEVMODEW);
            dm.dmFields |= DM_PAPERSIZE | DM_PAPERLENGTH | DM_PAPERWIDTH;
            dm.Anonymous1.Anonymous1.dmPaperSize   = DMPAPER_USER;
            dm.Anonymous1.Anonymous1.dmPaperWidth  = paper_w;
            dm.Anonymous1.Anonymous1.dmPaperLength = paper_h;

            Ok(buf)
        }
    }

    fn do_print(hdc: HDC, pngs: &[Vec<u8>]) -> Result<(), String> {
        let name_w: Vec<u16> = "Etiquetas".encode_utf16().chain(Some(0)).collect();
        let docinfo = DOCINFOW {
            cbSize: mem::size_of::<DOCINFOW>() as i32,
            lpszDocName: PCWSTR::from_raw(name_w.as_ptr()),
            lpszOutput: PCWSTR::null(),
            lpszDatatype: PCWSTR::null(),
            fwType: 0,
        };

        unsafe {
            if StartDocW(hdc, &docinfo) <= 0 {
                return Err("StartDocW falló".to_string());
            }
        }

        for (i, png) in pngs.iter().enumerate() {
            if let Err(e) = print_page(hdc, png) {
                unsafe { EndDoc(hdc) };
                return Err(format!("Página {i}: {e}"));
            }
        }

        unsafe { EndDoc(hdc) };
        Ok(())
    }

    fn print_page(hdc: HDC, png: &[u8]) -> Result<(), String> {
        use image::ImageReader;
        use std::io::Cursor;

        let img = ImageReader::new(Cursor::new(png))
            .with_guessed_format()
            .map_err(|e| e.to_string())?
            .decode()
            .map_err(|e| e.to_string())?;

        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();

        // RGBA → BGRA (orden de bytes de DIB en Win32)
        let mut bgra: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
        for px in rgba.pixels() {
            bgra.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
        }

        // Dimensiones de página en píxeles del dispositivo
        let page_w = unsafe { GetDeviceCaps(hdc, HORZRES) };
        let page_h = unsafe { GetDeviceCaps(hdc, VERTRES) };

        // biHeight negativo = DIB top-down (sin necesidad de invertir filas)
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize:          mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth:         w as i32,
                biHeight:        -(h as i32),
                biPlanes:        1,
                biBitCount:      32,
                biCompression:   BI_RGB.0,
                biSizeImage:     0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed:       0,
                biClrImportant:  0,
            },
            bmiColors: [RGBQUAD::default()],
        };

        unsafe {
            StartPage(hdc);
            StretchDIBits(
                hdc,
                0, 0, page_w, page_h,          // destino: página completa
                0, 0, w as i32, h as i32,       // fuente: imagen completa
                Some(bgra.as_ptr() as *const _),
                &bmi,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            EndPage(hdc);
        }

        Ok(())
    }
}
