use clap::{Arg, Command};
use std::fs;
use std::path::Path;
use std::process::Command as SysCommand;
use uuid::Uuid;

#[derive(Debug)]
enum Theme {
    Classic,
    Red,
    Photo(String),
}

fn main() {
    let matches = Command::new("mis-inst")
        .version("3.3.2")
        .author("mako")
        .about("Native Interactive MSI Installer Generator via WiX v4/v7")
        .arg(
            Arg::new("input")
                .help("Путь к исходному файлу дистрибутива")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("output")
                .help("Имя создаваемого файла MSI (например, DRON.msi)")
                .required(true)
                .index(2),
        )
        .arg(
            Arg::new("name")
                .help("Название приложения")
                .required(true)
                .index(3),
        )
        .arg(
            Arg::new("theme")
                .short('t')
                .long("theme")
                .num_args(1..)
                .help("Тема оформления: classic, red, photo <path>"),
        )
        .get_matches();

    let input_file = matches.get_one::<String>("input").unwrap();
    let output_file = matches.get_one::<String>("output").unwrap();
    let app_name = matches.get_one::<String>("name").unwrap();

    let theme = match matches.get_many::<String>("theme") {
        Some(mut values) => {
            let t_type = values.next().map(|s| s.as_str()).unwrap_or("classic");
            match t_type {
                "red" => Theme::Red,
                "photo" => {
                    if let Some(path) = values.next() {
                        Theme::Photo(path.to_string())
                    } else {
                        eprintln!("Ошибка: Для темы 'photo' укажите путь к картинке!");
                        std::process::exit(1);
                    }
                }
                _ => Theme::Classic,
            }
        }
        None => Theme::Classic,
    };

    println!("\n=== [MIS-INST 3.3.2] СБОРКА ИНТЕРАКТИВНОГО MSI С UI ===");
    println!(" Входной дистрибутив: {:?}", input_file);
    println!(" Создаваемый MSI:      {:?}", output_file);
    println!(" Имя продукта:        {}", app_name);
    println!(" Издатель:            My Developer Studio");
    println!(" Версия:              1.0.0");
    println!(" Тема оформления:      {:?}", theme);

    if !Path::new(input_file).exists() {
        eprintln!("\n Ошибка: Входной файл {:?} не найден!", input_file);
        std::process::exit(1);
    }

    let upgrade_code = Uuid::new_v4().to_string().to_uppercase();
    let comp_guid = Uuid::new_v4().to_string().to_uppercase();

    // Задаём Language="1049" и Codepage="1251" для поддержки кириллицы
    let wxs_content = format!(
        r#"<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs"
     xmlns:ui="http://wixtoolset.org/schemas/v4/wxs/ui">
  <Package Name="{app_name}"
           Manufacturer="My Developer Studio"
           Version="1.0.0"
           UpgradeCode="{upgrade_code}"
           Language="1049"
           Codepage="1251">

    <MediaTemplate EmbedCab="yes" />

    <MajorUpgrade DowngradeErrorMessage="Более новая версия [ProductName] уже установлена." />

    <ui:WixUI Id="WixUI_InstallDir" InstallDirectory="INSTALLFOLDER" />

    <StandardDirectory Id="ProgramFilesFolder">
      <Directory Id="INSTALLFOLDER" Name="{app_name}">
        <Component Id="MainExecutableComponent" Guid="{comp_guid}">
          <File Id="MainExecutableFile" Source="{input_file}" KeyPath="yes" />
        </Component>
      </Directory>
    </StandardDirectory>

    <Feature Id="MainFeature" Title="{app_name}" Level="1">
      <ComponentRef Id="MainExecutableComponent" />
    </Feature>
  </Package>
</Wix>"#,
        app_name = app_name,
        upgrade_code = upgrade_code,
        comp_guid = comp_guid,
        input_file = input_file
    );

    let wxs_path = "DRON.wxs";
    fs::write(wxs_path, wxs_content).expect("Не удалось записать .wxs манифест");
    println!("\n [Успех] Сформирован манифест с поддержкой кириллицы: {:?}", wxs_path);

    // Компиляция
    println!(" [Компиляция] Запуск 'wix build'...");
    let status = SysCommand::new("wix")
        .args(["build", wxs_path, "-ext", "WixToolset.UI.wixext", "-o", output_file])
        .status();

    match status {
        Ok(s) if s.success() => {
            let _ = fs::remove_file("DRON.wixpdb");
            println!("\n==========================================");
            println!(" УСПЕХ! ИНТЕРАКТИВНЫЙ MSI СОЗДАН: {:?}", output_file);
            println!("==========================================");
        }
        Ok(s) => {
            eprintln!("\n [Ошибка] Ошибка компиляции пакета (код: {})", s);
        }
        Err(e) => {
            eprintln!("\n [Ошибка] Не удалось запустить утилиту 'wix': {}", e);
        }
    }
}