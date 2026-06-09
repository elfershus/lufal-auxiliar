use base64::prelude::*;
use serde::Deserialize;
use std::process::Command;

#[derive(Deserialize)]
pub struct PngLabel {
    pub png_b64: String,
}

/// Lista las impresoras instaladas usando PowerShell Get-Printer.
#[tauri::command]
pub fn list_printers() -> Vec<String> {
    #[cfg(not(windows))]
    return vec![];

    #[cfg(windows)]
    {
        let Ok(out) = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                // @() fuerza array JSON incluso con un solo elemento
                "@(Get-Printer | Select-Object -ExpandProperty Name) | ConvertTo-Json -Compress",
            ])
            .output()
        else {
            return vec![];
        };

        let stdout = String::from_utf8_lossy(&out.stdout);
        serde_json::from_str(stdout.trim()).unwrap_or_default()
    }
}

/// Imprime etiquetas PNG en la impresora indicada usando
/// System.Drawing.Printing de .NET Framework vía PowerShell.
/// No contiene ningún bloque unsafe.
#[tauri::command]
pub fn print_etiquetas(
    labels: Vec<PngLabel>,
    height_mm: f32,
    printer_name: String,
) -> Result<(), String> {
    if labels.is_empty() {
        return Ok(());
    }

    #[cfg(not(windows))]
    return Err("Impresión directa solo disponible en Windows".to_string());

    #[cfg(windows)]
    {
        // ── Guardar PNGs en directorio temporal ───────────────────
        let temp_dir = std::env::temp_dir().join("lufal_etiquetas");
        std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

        let count = labels.len();
        for (i, label) in labels.iter().enumerate() {
            let bytes = BASE64_STANDARD
                .decode(&label.png_b64)
                .map_err(|e| format!("base64 label {i}: {e}"))?;
            std::fs::write(temp_dir.join(format!("label_{i}.png")), bytes)
                .map_err(|e| e.to_string())?;
        }

        // ── Generar script PowerShell ─────────────────────────────
        // Dimensiones en centésimos de pulgada (unidad de PaperSize)
        let paper_w = (62.0_f32 / 25.4 * 100.0).round() as i32; // 62 mm
        let paper_h = (height_mm / 25.4 * 100.0).round() as i32;

        // Escapar comillas simples para strings PS
        let printer_ps = printer_name.replace('\'', "''");
        let dir_ps = temp_dir.to_string_lossy().replace('\'', "''");

        // Nota: {{ y }} son literales { y } en format!
        let script = format!(
            "Add-Type -AssemblyName System.Drawing\n\
$printer = '{printer}'\n\
$pw = {w}\n\
$ph = {h}\n\
for ($i = 0; $i -lt {count}; $i++) {{\n\
    $path = '{dir}\\label_' + $i + '.png'\n\
    $img  = [System.Drawing.Bitmap]::FromFile($path)\n\
    $doc  = New-Object System.Drawing.Printing.PrintDocument\n\
    $doc.PrinterSettings.PrinterName    = $printer\n\
    $doc.DefaultPageSettings.PaperSize = New-Object System.Drawing.Printing.PaperSize('Custom', $pw, $ph)\n\
    $doc.DefaultPageSettings.Margins   = New-Object System.Drawing.Printing.Margins(0, 0, 0, 0)\n\
    $ref = $img\n\
    $doc.add_PrintPage({{\n\
        param($s, $e)\n\
        $e.Graphics.DrawImage($ref, 0, 0, $e.PageBounds.Width, $e.PageBounds.Height)\n\
        $e.HasMorePages = $false\n\
    }})\n\
    $doc.Print()\n\
    $doc.Dispose()\n\
    $img.Dispose()\n\
}}\n",
            printer = printer_ps,
            w = paper_w,
            h = paper_h,
            count = count,
            dir = dir_ps,
        );

        let script_path = temp_dir.join("print_labels.ps1");
        std::fs::write(&script_path, &script).map_err(|e| e.to_string())?;

        // ── Ejecutar script ───────────────────────────────────────
        let out = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                &script_path.to_string_lossy(),
            ])
            .output()
            .map_err(|e| e.to_string())?;

        // ── Limpiar temporales ────────────────────────────────────
        for i in 0..count {
            let _ = std::fs::remove_file(temp_dir.join(format!("label_{i}.png")));
        }
        let _ = std::fs::remove_file(&script_path);

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                format!("PowerShell terminó con código {:?}", out.status.code())
            } else {
                stderr
            });
        }

        Ok(())
    }
}
