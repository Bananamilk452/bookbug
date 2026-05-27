use crate::utils::Error;

#[tauri::command]
pub fn test(input: String) -> Result<String, Error> {
  if input == "hello" {
    Ok("world".to_string())
  } else {
    Err(Error::Io(std::io::Error::new(
      std::io::ErrorKind::InvalidInput,
      "input must be 'hello'",
    )))
  }
}