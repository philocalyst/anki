fn main() {
    let example_content = include_str!("/home/miles/Downloads/oh/example.flash");

    let mut current_note_model = "".to_string();

    let mut question_field = "".to_string();
    let mut answer_field = "".to_string();

    for line in example_content.lines() {

        if line.contains("=") {
            current_note_model = line.replace("=", "");
        } else if line.starts_with("Question:") {
            question_field = line.replace("Question:", "");
        } else if line.starts_with("Answer:") {
            answer_field = line.replace("Answer:", "");
        } else {
            continue;
        }
        
        println!("{}{}{}", current_note_model, question_field, answer_field);
    }
}
