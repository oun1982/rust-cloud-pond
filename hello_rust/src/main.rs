  struct User{
        name: String,
        age: u32,
        is_student: bool,  
    }

    struct Book{
        title: String,
        pages: u32,
        is_available: bool,
    }


    impl Book{
        fn print_title(&self){
            println!("Book Title: {}", self.title);
        }
    }

    impl Book {
    fn borrow(&mut self) {
        self.is_available = false;
    }
}

fn main(){
    let user = User {
        name: String::from("Pongsakon"),
        age: 44,
        is_student: true,
    };

    let mut book = Book {
        title: String::from("Rust Programming"),
        pages: 300,
        is_available: true,
    };

    println!("Name: {} Age: {} Student: {}", user.name, user.age, user.is_student);
    
    book.is_available = false; // This line will cause a compile-time error because `book` is not mutable.
    println!("Title: {} Pages: {} Available: {}", book.title, book.pages, book.is_available);

    book.print_title();


}