use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(
    name = "mis-inst",
    version = "3.5.2",
    about = "mis Installer Generator — нативная сборка MSI через WiX Toolset"
)]
struct Args {
    /// Путь к основному исполняемому файлу дистрибутива
    input: PathBuf,

    /// Имя или путь для итогового MSI пакета
    output: String,

    /// Название приложения
    name: String,

    /// Издатель / Разработчик (-d)
    #[arg(short = 'd', long, default_value = "Developer Studio")]
    developer: String,

    /// Тема оформления: classic (классическая синяя), red (красная) или photo <путь_к_картинке>
    #[arg(short = 't', long, default_value = "classic", num_args = 1..=2)]
    theme: Vec<String>,

    /// Дополнительные файлы дистрибутива (-e <файл1> <файл2> ...)
    #[arg(short = 'e', long = "extra", value_name = "FILES", num_args = 1..)]
    extra: Vec<PathBuf>,
}

/// Генератор 24-bit BMP изображений
fn save_bmp(path: &Path, width: u32, height: u32, pixels: &[(u8, u8, u8)]) -> std::io::Result<()> {
    let row_size = ((width * 3 + 3) / 4) * 4;
    let pixel_array_size = row_size * height;
    let file_size = 54 + pixel_array_size;

    let mut data = Vec::with_capacity(file_size as usize);

    // 14 bytes BMP Header
    data.extend_from_slice(b"BM");
    data.extend_from_slice(&(file_size as u32).to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&54u32.to_le_bytes());

    // 40 bytes DIB Header
    data.extend_from_slice(&40u32.to_le_bytes());
    data.extend_from_slice(&(width as i32).to_le_bytes());
    data.extend_from_slice(&(height as i32).to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&24u16.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&(pixel_array_size as u32).to_le_bytes());
    data.extend_from_slice(&2835i32.to_le_bytes());
    data.extend_from_slice(&2835i32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());

    // Write pixels (bottom-up)
    for y in 0..height {
        let mut row_bytes = 0;
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let (r, g, b) = pixels[idx];
            data.push(b);
            data.push(g);
            data.push(r);
            row_bytes += 3;
        }
        while row_bytes % 4 != 0 {
            data.push(0);
            row_bytes += 1;
        }
    }

    fs::write(path, data)
}

/// Генерация классической синей обложки WiX v3 / Advanced Installer (dlgbmp.bmp)
fn generate_classic_dialog_bmp(path: &Path) -> std::io::Result<()> {
    let width = 493u32;
    let height = 312u32;
    let mut pixels = vec![(255u8, 255u8, 255u8); (width * height) as usize];

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;

            if x <= 164 {
                // Левая синяя панель
                let mut r = 0u8;
                let mut g = 0u8;
                let mut b = 100u8; // Тёмно-синий базовый фон

                // Контуры кругов диска на фоне
                let dx = x as f32 - 10.0;
                let dy = y as f32 - 120.0;
                let dist = (dx * dx + dy * dy).sqrt();

                if (dist - 145.0).abs() < 10.0 || (dist - 90.0).abs() < 7.0 || (dist - 40.0).abs() < 4.0 {
                    b = 180; // Ярко-синяя линия диска
                }

                // Белый квадрат с логотипом диска (в верхней части панели)
                // y в BMP отсчитывается снизу вверх (y=200..275 — это верх)
                if x >= 45 && x <= 120 && y >= 200 && y <= 275 {
                    if x <= 48 || x >= 117 || y <= 203 || y >= 272 {
                        // Белая рамка
                        r = 255; g = 255; b = 255;
                    } else {
                        // Иконка диска внутри квадрата
                        let cx = 82.5f32;
                        let cy = 237.5f32;
                        let ic_dx = x as f32 - cx;
                        let ic_dy = y as f32 - cy;
                        let ic_dist = (ic_dx * ic_dx + ic_dy * ic_dy).sqrt();

                        if ic_dist <= 25.0 {
                            if ic_dist >= 21.0 || ic_dist <= 6.0 || (ic_dx.abs() < 3.0 && ic_dy.abs() < 3.0) {
                                r = 255; g = 255; b = 255;
                            } else if (ic_dx - ic_dy).abs() < 3.0 {
                                r = 210; g = 210; b = 255; // Блик на диске
                            }
                        }
                    }
                }

                pixels[idx] = (r, g, b);
            } else {
                // Правая область для текста — чисто белая
                pixels[idx] = (255, 255, 255);
            }
        }
    }

    save_bmp(path, width, height, &pixels)
}

/// Генерация верхнего баннера (bannrbmp.bmp 493x58)
fn generate_classic_banner_bmp(path: &Path) -> std::io::Result<()> {
    let width = 493u32;
    let height = 58u32;
    let mut pixels = vec![(255u8, 255u8, 255u8); (width * height) as usize];

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            if x >= 420 {
                // Небольшой градиент справа
                let factor = (x - 420) as f32 / 73.0;
                let b = (255.0 - factor * 155.0) as u8;
                pixels[idx] = (0, 0, b);
            } else {
                pixels[idx] = (255, 255, 255);
            }
        }
    }

    save_bmp(path, width, height, &pixels)
}

fn main() {
    let args = Args::parse();

    // 1. Определение темы
    let theme_mode = args.theme.first().map(|s| s.as_str()).unwrap_or("classic");

    let (color_code, reset_code) = match theme_mode {
        "red" => ("\x1b[91m", "\x1b[0m"),
        "classic" | _ => ("\x1b[94m", "\x1b[0m"),
    };

    println!("{}=== [MIS-INST v3.5.2] СБОРКА MSI ==={}", color_code, reset_code);
    println!("  Основной файл:   {:?}", args.input);
    println!("  Создаваемый MSI: \"{}\"", args.output);
    println!("  Имя продукта:    \"{}\"", args.name);
    println!("  Издатель (-d):   \"{}\"", args.developer);
    println!("  Тема (-t):       {}", theme_mode);

    if !args.extra.is_empty() {
        println!("  Доп. файлы (-e): {} шт.", args.extra.len());
        for file in &args.extra {
            println!("    └─ {:?}", file);
        }
    }

    // 2. Имена временных файлов
    let output_path = Path::new(&args.output);
    let file_stem = output_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("installer_manifest");

    let wxs_filename = format!("{}.wxs", file_stem);
    let temp_dialog_bmp = PathBuf::from(format!("temp_dialog_{}.bmp", file_stem));
    let temp_banner_bmp = PathBuf::from(format!("temp_banner_{}.bmp", file_stem));

    let mut generated_bmps = Vec::new();

    // 3. Формирование темы
    let wix_ui_variables = match theme_mode {
        "red" => {
            // Для красной темы используем встроенные ресурсы WiX v4
            format!(
                r#"<!-- Red Theme Configuration -->
        <WixVariable Id="WixUIExclamationIco" Value="C:\Windows\System32\shell32.dll,14" />"#
            )
        }
        "photo" => {
            if let Some(photo_path) = args.theme.get(1) {
                format!(
                    r#"<!-- Custom Photo Theme -->
        <WixVariable Id="WixUIBannerBmp" Value="{}" />
        <WixVariable Id="WixUIDialogBmp" Value="{}" />"#,
                    photo_path, photo_path
                )
            } else {
                println!("  [!] Для темы 'photo' не указан путь к файлу. Используется 'classic'.");
                String::new()
            }
        }
        "classic" | _ => {
            // Генерируем классическую синюю обложку WiX v3 / Advanced Installer
            let _ = generate_classic_dialog_bmp(&temp_dialog_bmp);
            let _ = generate_classic_banner_bmp(&temp_banner_bmp);
            generated_bmps.push(&temp_dialog_bmp);
            generated_bmps.push(&temp_banner_bmp);

            format!(
                r#"<!-- Classic Blue Theme Configuration (Advanced Installer / WiX 3 style) -->
        <WixVariable Id="WixUIDialogBmp" Value="{}" />
        <WixVariable Id="WixUIBannerBmp" Value="{}" />"#,
                temp_dialog_bmp.display(),
                temp_banner_bmp.display()
            )
        }
    };

    // 4. Генерация GUID
    let upgrade_code = Uuid::new_v4().to_string().to_uppercase();
    let main_comp_guid = Uuid::new_v4().to_string().to_uppercase();

    // 5. Генерация компонентов доп. файлов
    let mut extra_components_xml = String::new();
    for (idx, extra_path) in args.extra.iter().enumerate() {
        let extra_guid = Uuid::new_v4().to_string().to_uppercase();
        let extra_str = extra_path.to_string_lossy();

        extra_components_xml.push_str(&format!(
            r#"
                <Component Id="ExtraFile_{idx}" Guid="{extra_guid}">
                    <File Source="{extra_str}" />
                </Component>"#,
            idx = idx + 1,
            extra_guid = extra_guid,
            extra_str = extra_str
        ));
    }

    // 6. Запись .wxs
    let input_path_str = args.input.to_string_lossy();
    let wxs_content = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs"
     xmlns:ui="http://wixtoolset.org/schemas/v4/wxs/ui">
    <Package Name="{product_name}"
             Manufacturer="{developer}"
             Version="1.0.0.0"
             UpgradeCode="{upgrade_code}"
             Language="1049"
             Codepage="1251">

        <MajorUpgrade DowngradeErrorMessage="Более новая версия программы уже установлена." />
        <MediaTemplate EmbedCab="yes" />

        {wix_ui_variables}

        <ui:WixUI Id="WixUI_InstallDir" InstallDirectory="INSTALLFOLDER" />

        <StandardDirectory Id="ProgramFiles64Folder">
            <Directory Id="INSTALLFOLDER" Name="{product_name}">
                <Component Id="MainExecutableComponent" Guid="{main_comp_guid}">
                    <File Source="{input_path}" KeyPath="yes" />
                </Component>{extra_components}
            </Directory>
        </StandardDirectory>
    </Package>
</Wix>"#,
        product_name = args.name,
        developer = args.developer,
        upgrade_code = upgrade_code,
        wix_ui_variables = wix_ui_variables,
        main_comp_guid = main_comp_guid,
        input_path = input_path_str,
        extra_components = extra_components_xml
    );

    if let Err(e) = fs::write(&wxs_filename, wxs_content) {
        eprintln!("[Ошибка] Не удалось записать манифест {}: {}", wxs_filename, e);
        return;
    }

    println!("  [Успех] Сформирован манифест: \"{}\"", wxs_filename);
    println!("  [Компиляция] Запуск 'wix build'...");

    // 7. Сборка пакета через WiX
    let status = Command::new("wix")
        .arg("build")
        .arg(&wxs_filename)
        .arg("-ext")
        .arg("WixToolset.UI.wixext")
        .arg("-o")
        .arg(&args.output)
        .status();

    // 8. Очистка временных файлов
    let _ = fs::remove_file(&wxs_filename);
    let _ = fs::remove_file(format!("{}.wixpdb", file_stem));
    for bmp in generated_bmps {
        let _ = fs::remove_file(bmp);
    }

    match status {
        Ok(s) if s.success() => {
            println!("\n{}=========================================={}", color_code, reset_code);
            println!("{}  УСПЕХ! Собрано: \"{}\"{}", color_code, args.output, reset_code);
            println!("{}=========================================={}", color_code, reset_code);
        }
        Ok(s) => eprintln!("[Ошибка] WiX завершился с кодом: {}", s),
        Err(e) => eprintln!("[Ошибка] Не удалось запустить WiX Toolset: {}", e),
    }
}