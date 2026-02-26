import type { Guide } from './types'

export const rustGuide: Guide = {
  id: 'guide-rust',
  title: 'Rust 101',
  subtitle: 'Build a mini gem notebook from scratch — learn Rust by building, not by reading definitions',
  color: 'orange',
  icon: '🦀',
  sections: [
    // ── 0. The Big Picture ──
    {
      id: 'big-picture',
      title: 'What Are We Building?',
      content: [
        {
          type: 'text',
          body: 'We\'re going to build a tiny "gem notebook" — a command-line app where you can save notes, tag them, and search through them. It\'s a miniature version of what Jarvis does. Along the way, you\'ll learn Rust not by memorizing definitions, but by needing each concept to build the next feature.',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'setup', label: '1. Set up project', icon: '🔧' },
            { id: 'first', label: '2. First program', icon: '👋' },
            { id: 'data', label: '3. Store data', icon: '📦' },
            { id: 'own', label: '4. Ownership', icon: '🔑' },
            { id: 'struct', label: '5. Build a Gem', icon: '💎' },
            { id: 'err', label: '6. Handle errors', icon: '🛡️' },
            { id: 'trait', label: '7. Shared behavior', icon: '🤝' },
            { id: 'async', label: '8. Do many things', icon: '⚡' },
          ],
          connections: [
            { from: 'setup', to: 'first' },
            { from: 'first', to: 'data' },
            { from: 'data', to: 'own' },
            { from: 'own', to: 'struct' },
            { from: 'struct', to: 'err' },
            { from: 'err', to: 'trait' },
            { from: 'trait', to: 'async' },
          ],
        },
        {
          type: 'text',
          body: 'Each step builds on the last. By the end, you\'ll understand every Rust concept Jarvis uses — because you\'ll have used them yourself.',
        },
      ],
    },

    // ── 1. Setting Up Your Workshop ──
    {
      id: 'setup',
      title: 'Step 1: Setting Up Your Workshop',
      content: [
        {
          type: 'text',
          body: 'Before you can cook, you need a kitchen. Before you can build with Rust, you need to install it and create a project. This takes about 2 minutes.',
        },
        {
          type: 'code',
          language: 'bash',
          code: '# Install Rust (one command — it sets up everything)\ncurl --proto \'=https\' --tlsv1.2 -sSf https://sh.rustup.rs | sh\n\n# Verify it worked\nrustc --version\ncargo --version',
          caption: 'Run this in your terminal. Cargo is Rust\'s tool for building projects — think of it as npm for Rust.',
        },
        {
          type: 'text',
          body: 'Now create your project. Cargo sets up the folders, creates a starter file, and handles all the boring stuff.',
        },
        {
          type: 'code',
          language: 'bash',
          code: '# Create a new project called "gem-notebook"\ncargo new gem-notebook\ncd gem-notebook\n\n# Run it!\ncargo run',
          caption: 'You should see "Hello, world!" — congratulations, you just compiled and ran a Rust program.',
        },
        {
          type: 'text',
          body: 'What just happened? Unlike Python or JavaScript where you run code directly, Rust has a "compile" step. Think of it like this: the compiler reads your recipe, checks it for mistakes, and creates a finished dish (a program) that runs incredibly fast. If the recipe has ANY mistake, it won\'t even start cooking — that\'s Rust\'s safety guarantee.',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'code', label: 'Your code (.rs)', icon: '📝' },
            { id: 'compiler', label: 'Compiler checks it', icon: '🔍' },
            { id: 'binary', label: 'Fast program', icon: '🚀' },
          ],
          connections: [
            { from: 'code', to: 'compiler', label: 'cargo build' },
            { from: 'compiler', to: 'binary', label: 'no errors? → output' },
          ],
        },
        {
          type: 'text',
          body: 'Open the file `src/main.rs` — that\'s where your code lives. You\'ll see one function: `fn main()`. This is the starting point. When you run the program, Rust executes whatever is inside `main`.',
        },
        {
          type: 'quiz',
          question: 'What does `cargo run` do?',
          options: [
            'Runs the code directly like Python',
            'Compiles the code into a program, then runs that program',
            'Uploads the code to the cloud',
            'Opens the code in an editor',
          ],
          correctIndex: 1,
          explanation: 'Rust is a compiled language. `cargo run` first compiles your .rs files into a native binary (machine code), then executes it. This compilation step is why Rust catches so many bugs before your program even runs.',
        },
      ],
    },

    // ── 2. Your First Lines — Variables and Printing ──
    {
      id: 'first-lines',
      title: 'Step 2: Variables — Labeling Your Boxes',
      content: [
        {
          type: 'text',
          body: 'Imagine you\'re moving houses. You pack things into boxes and put labels on them: "Kitchen stuff", "Books", "Fragile". A variable in programming is exactly that — a labeled box. You put a value inside, and refer to it by the label.',
        },
        {
          type: 'code',
          language: 'rust',
          code: 'fn main() {\n    // Create a labeled box called "title" with text inside\n    let title = "My first gem";\n    \n    // Create a box called "importance" with a number\n    let importance = 5;\n    \n    // Print them out\n    println!("Title: {}", title);\n    println!("Importance: {}/10", importance);\n}',
          caption: 'Replace the contents of src/main.rs with this, then run `cargo run`',
        },
        {
          type: 'text',
          body: 'Notice something: we used `let` (not `var`, `const`, or anything else). In Rust, `let` creates a variable that CANNOT be changed. It\'s like writing on a box with permanent marker. Why? Because if things can\'t change unexpectedly, your program has fewer surprises.',
        },
        {
          type: 'code',
          language: 'rust',
          code: 'fn main() {\n    let title = "My gem";\n    title = "Changed!"; // ERROR! Can\'t change a `let` variable\n    \n    // If you WANT to change it, say so explicitly:\n    let mut title = "My gem";  // `mut` = "I plan to change this"\n    title = "Changed!";        // Now it\'s OK\n    println!("{}", title);\n}',
          caption: '`mut` is short for "mutable" — meaning changeable. Rust makes you be explicit about what can change.',
        },
        {
          type: 'concept-card',
          term: 'let',
          explanation: 'Creates a variable that cannot be changed. Like writing a label in permanent marker. Most variables should be `let` — fewer moving parts means fewer bugs.',
          example: 'let name = "Jarvis";\nlet version = 2;',
        },
        {
          type: 'concept-card',
          term: 'let mut',
          explanation: 'Creates a variable that CAN be changed. Like writing a label in pencil. Use this when you know the value will need to be updated later.',
          example: 'let mut count = 0;\ncount = count + 1; // OK!',
        },
        {
          type: 'concept-card',
          term: 'println!("...", value)',
          explanation: 'Prints text to the screen. The {} is a placeholder that gets replaced with the value. The ! means it\'s a "macro" (a shortcut that writes more code for you — don\'t worry about the details yet).',
          example: 'println!("Hello, {}!", "world");\n// Output: Hello, world!',
        },
        {
          type: 'interactive-code',
          language: 'rust',
          starterCode: '// Create two variables:\n// 1. A `let` variable called `gem_title` with any text\n// 2. A `let mut` variable called `tag_count` starting at 0\n// Then change tag_count to 3\n// Print: "Gem: [title] has [count] tags"\n\nfn main() {\n    // your code here\n}',
          solution: 'fn main() {\n    let gem_title = "Meeting notes";\n    let mut tag_count = 0;\n    tag_count = 3;\n    println!("Gem: {} has {} tags", gem_title, tag_count);\n}',
          hint: 'Use `let` for the title (it won\'t change) and `let mut` for the count (it will). Use println! with {} placeholders.',
          validator: (input: string) => {
            return input.includes('let gem_title') &&
              input.includes('let mut tag_count') &&
              input.includes('println!')
          },
        },
      ],
    },

    // ── 3. Collections — A Box of Many Things ──
    {
      id: 'collections',
      title: 'Step 3: Collections — Your List of Gems',
      content: [
        {
          type: 'text',
          body: 'One box is nice, but our notebook needs to hold MANY gems. Imagine a filing cabinet drawer — you can add files, count them, and flip through them. In Rust, this is called a `Vec` (short for "vector" — just a fancy name for a list that can grow).',
        },
        {
          type: 'code',
          language: 'rust',
          code: 'fn main() {\n    // Create an empty filing cabinet (list)\n    let mut gems = Vec::new();\n    \n    // Add some gems\n    gems.push("Meeting with Alice");\n    gems.push("Recipe for pasta");\n    gems.push("Rust learning notes");\n    \n    // How many do we have?\n    println!("You have {} gems", gems.len());\n    \n    // Look at the first one\n    println!("First gem: {}", gems[0]);\n    \n    // Go through ALL of them\n    for gem in &gems {\n        println!("  - {}", gem);\n    }\n}',
          caption: 'A Vec is like a list in Python or an array in JavaScript — but it must hold items of the same type',
        },
        {
          type: 'text',
          body: 'Notice the `&` before `gems` in the for loop? That\'s a preview of something big. It means "let me LOOK at the gems without taking them." We\'ll understand why this matters in the next step. For now, just know: `&` means "I\'m borrowing this, not taking it."',
        },
        {
          type: 'concept-card',
          term: 'Vec<T>',
          explanation: 'A growable list. T is the type of items inside. Vec<String> holds text, Vec<i32> holds whole numbers. You can push items in, remove them, count them, and loop through them.',
          example: 'let mut numbers: Vec<i32> = Vec::new();\nnumbers.push(10);\nnumbers.push(20);\n// numbers = [10, 20]',
        },
        {
          type: 'concept-card',
          term: 'String vs &str',
          explanation: 'Rust has two kinds of text. String is a box of text you OWN (can change it, grow it). &str is a WINDOW into text someone else owns (read-only). "hello" is &str. String::from("hello") is String.',
          example: 'let borrowed: &str = "hello";       // just looking\nlet owned: String = String::from("hello"); // I own this',
        },
        {
          type: 'interactive-code',
          language: 'rust',
          starterCode: '// Build a list of 3 tags for a gem\n// Then print how many tags there are\n// Then print each tag on its own line\n\nfn main() {\n    // your code here\n}',
          solution: 'fn main() {\n    let mut tags = Vec::new();\n    tags.push("work");\n    tags.push("important");\n    tags.push("meeting");\n    println!("{} tags:", tags.len());\n    for tag in &tags {\n        println!("  #{}", tag);\n    }\n}',
          hint: 'Create a `let mut tags = Vec::new()`, push 3 strings in, use .len() to count, and a for loop to print each one.',
          validator: (input: string) => {
            return input.includes('Vec::new()') &&
              input.includes('push(') &&
              input.includes('.len()') &&
              input.includes('for ')
          },
        },
        {
          type: 'quiz',
          question: 'Why does the list need to be `let mut`?',
          options: [
            'Because Rust requires all variables to be mut',
            'Because we\'re adding items to it — that changes it',
            'Because Vec is always mutable',
            'It doesn\'t — let would work fine',
          ],
          correctIndex: 1,
          explanation: 'When you push() an item into a Vec, you\'re changing the list. Rust requires you to declare that intent upfront with `mut`. If you try to push into a `let` Vec, the compiler will refuse — it\'s protecting you from accidental changes.',
        },
      ],
    },

    // ── 4. Ownership — The Big Idea ──
    {
      id: 'ownership',
      title: 'Step 4: Ownership — Why Rust Is Different',
      content: [
        {
          type: 'text',
          body: 'This is the ONE concept that makes Rust different from every other language. It takes 10 minutes to understand and saves you from entire categories of bugs. Let\'s start with a story.',
        },
        {
          type: 'text',
          body: 'Imagine you have a physical notebook — one single copy. You give it to your friend Alice. Now YOU don\'t have it anymore. You can\'t write in it. You can\'t read it. Alice has it. This is OWNERSHIP in Rust. When you give a value to someone else, you no longer have it.',
        },
        {
          type: 'code',
          language: 'rust',
          code: 'fn main() {\n    let notebook = String::from("My precious notes");\n    \n    let alice = notebook;  // Alice now OWNS the notebook\n    \n    // This would FAIL — you gave it away!\n    // println!("{}", notebook);\n    \n    // Only Alice can use it now\n    println!("{}", alice);\n}',
          caption: 'Try uncommenting the println — the compiler will tell you: "value used after move"',
        },
        {
          type: 'text',
          body: 'But wait — what if Alice just needs to READ your notebook for a minute? You don\'t want to give it away permanently. You want to LEND it. In Rust, this is called "borrowing" and you write it with `&`.',
        },
        {
          type: 'code',
          language: 'rust',
          code: 'fn main() {\n    let notebook = String::from("My precious notes");\n    \n    // LEND the notebook (borrow with &)\n    let alice = &notebook;\n    \n    // You STILL have it!\n    println!("Mine: {}", notebook);\n    println!("Alice sees: {}", alice);\n}',
          caption: '& means "borrow" — Alice can look, but you still own it',
        },
        {
          type: 'text',
          body: 'And what if Alice needs to WRITE in your notebook? That\'s more risky — what if someone else is reading it at the same time? Rust has a rule: only ONE person can write at a time. You express this with `&mut`.',
        },
        {
          type: 'code',
          language: 'rust',
          code: 'fn main() {\n    let mut notebook = String::from("My notes");\n    \n    // Lend it for writing (only one at a time!)\n    let alice = &mut notebook;\n    alice.push_str(" + Alice\'s addition");\n    \n    println!("{}", alice);\n}',
          caption: '&mut means "borrow for writing" — only one borrower can write at a time',
        },
        {
          type: 'concept-card',
          term: 'Move (give away)',
          explanation: 'When you assign a value to another variable (or pass it to a function), the original can\'t use it anymore. Like handing someone your physical notebook.',
          example: 'let a = String::from("hello");\nlet b = a;  // a is GONE\n// a is no longer valid here',
        },
        {
          type: 'concept-card',
          term: 'Borrow (&)',
          explanation: 'Lend your value temporarily. The borrower can LOOK but not change. Multiple borrowers can look at the same time.',
          example: 'let a = String::from("hello");\nlet b = &a; // borrowing\nprintln!("{} {}", a, b); // both work!',
        },
        {
          type: 'concept-card',
          term: 'Mutable borrow (&mut)',
          explanation: 'Lend your value for WRITING. Only one person can do this at a time. And nobody else can even read while someone is writing.',
          example: 'let mut a = String::from("hello");\nlet b = &mut a;\nb.push_str(" world");',
        },
        {
          type: 'text',
          body: 'Why does this matter? In other languages, two parts of your program can modify the same data at the same time. This causes bugs that are INCREDIBLY hard to find — data corruption, crashes, security holes. Rust makes these bugs IMPOSSIBLE by checking ownership at compile time. Your program won\'t even build if there\'s a conflict.',
        },
        {
          type: 'comparison',
          leftLabel: 'Other languages',
          rightLabel: 'Rust',
          rows: [
            { label: 'Who owns data?', left: 'Anyone, nobody checks', right: 'Exactly one owner, enforced' },
            { label: 'Two writers?', left: 'Allowed (causes bugs)', right: 'Compiler error (prevented)' },
            { label: 'Use after delete?', left: 'Crash at runtime', right: 'Compiler error (prevented)' },
            { label: 'Memory cleanup?', left: 'Garbage collector (slow) or manual (error-prone)', right: 'Automatic when owner goes away' },
            { label: 'When bugs found?', left: 'When users complain', right: 'When you try to compile' },
          ],
        },
        {
          type: 'interactive-code',
          language: 'rust',
          starterCode: '// This code is broken. Fix it using borrowing (&)\n// The goal: both print statements should work\n\nfn main() {\n    let gem = String::from("Important meeting notes");\n    let backup = gem;\n    println!("Original: {}", gem);\n    println!("Backup: {}", backup);\n}',
          solution: 'fn main() {\n    let gem = String::from("Important meeting notes");\n    let backup = &gem;\n    println!("Original: {}", gem);\n    println!("Backup: {}", backup);\n}',
          hint: 'The problem is `let backup = gem` — this MOVES the value. Change it to `let backup = &gem` to BORROW instead.',
          validator: (input: string) => {
            return input.includes('&gem') && input.includes('println!') && !input.includes('.clone()')
          },
        },
        {
          type: 'quiz',
          question: 'You have `let notebook = String::from("notes");` and then `let copy = notebook;`. What happens if you try to use `notebook` after this?',
          options: [
            'It works fine — both variables have the value',
            'Compiler error — notebook was "moved" to copy and is no longer valid',
            'Runtime crash',
            'It prints an empty string',
          ],
          correctIndex: 1,
          explanation: 'This is Rust\'s ownership in action. When you assign notebook to copy, the ownership MOVES. notebook is no longer valid. The compiler catches this before your program even runs. To keep both, use borrowing: `let copy = &notebook;`',
        },
      ],
    },

    // ── 5. Structs — Building a Real Gem ──
    {
      id: 'structs',
      title: 'Step 5: Structs — Designing Your Gem',
      content: [
        {
          type: 'text',
          body: 'So far our gems are just text strings. But a real gem has a title, content, tags, and maybe a rating. We need a way to group related information together — like a form with multiple fields. In Rust, this is called a `struct` (short for "structure").',
        },
        {
          type: 'text',
          body: 'Think of a struct like a template for an ID card. The template says "every card has a Name, Photo, and Birthday." Each actual card fills in those fields differently. The struct defines the shape; each instance fills it in.',
        },
        {
          type: 'code',
          language: 'rust',
          code: '// The TEMPLATE: what does a Gem look like?\nstruct Gem {\n    title: String,\n    content: String,\n    tags: Vec<String>,\n}\n\nfn main() {\n    // Create an actual gem from the template\n    let gem = Gem {\n        title: String::from("Rust ownership"),\n        content: String::from("Every value has exactly one owner"),\n        tags: vec![String::from("rust"), String::from("learning")],\n    };\n    \n    println!("Gem: {}", gem.title);\n    println!("Tags: {:?}", gem.tags);\n}',
          caption: 'A struct groups related data together. Access fields with dot notation: gem.title',
        },
        {
          type: 'text',
          body: 'Now let\'s attach BEHAVIOR to our gem. What can a gem DO? It can describe itself, count its tags, add a tag. We put these actions in an `impl` block ("implementation"). Think of the struct as the blueprint and impl as the instruction manual.',
        },
        {
          type: 'code',
          language: 'rust',
          code: 'struct Gem {\n    title: String,\n    content: String,\n    tags: Vec<String>,\n}\n\n// The INSTRUCTION MANUAL for Gem\nimpl Gem {\n    // Create a new gem (like a factory)\n    fn new(title: &str, content: &str) -> Gem {\n        Gem {\n            title: String::from(title),\n            content: String::from(content),\n            tags: Vec::new(),\n        }\n    }\n    \n    // Describe yourself (&self = "look at me but don\'t change me")\n    fn summary(&self) -> String {\n        format!("{} ({} tags)", self.title, self.tags.len())\n    }\n    \n    // Add a tag (&mut self = "I need to change myself")\n    fn add_tag(&mut self, tag: &str) {\n        self.tags.push(String::from(tag));\n    }\n}\n\nfn main() {\n    let mut gem = Gem::new("Rust notes", "Learning ownership today");\n    gem.add_tag("rust");\n    gem.add_tag("learning");\n    println!("{}", gem.summary());\n    // Output: Rust notes (2 tags)\n}',
          caption: 'Notice: summary uses &self (read-only) but add_tag uses &mut self (needs to modify). Ownership in action!',
        },
        {
          type: 'concept-card',
          term: '&self',
          explanation: 'Used in methods that only LOOK at the struct\'s data. Borrows the struct as read-only. Most methods use this.',
          example: 'fn summary(&self) -> String {\n    // can read self.title but can\'t change it\n}',
        },
        {
          type: 'concept-card',
          term: '&mut self',
          explanation: 'Used in methods that CHANGE the struct\'s data. Borrows the struct for writing. The instance must be declared with `let mut`.',
          example: 'fn add_tag(&mut self, tag: &str) {\n    self.tags.push(String::from(tag));\n}',
        },
        {
          type: 'interactive-code',
          language: 'rust',
          starterCode: '// Add a method called `is_tagged` to Gem\n// It takes &self and a tag: &str\n// Returns true if self.tags contains that tag\n// Hint: self.tags.contains(&String::from(tag))\n\nstruct Gem {\n    title: String,\n    tags: Vec<String>,\n}\n\nimpl Gem {\n    fn new(title: &str) -> Gem {\n        Gem { title: String::from(title), tags: Vec::new() }\n    }\n    \n    fn add_tag(&mut self, tag: &str) {\n        self.tags.push(String::from(tag));\n    }\n    \n    // Add is_tagged here\n}',
          solution: 'struct Gem {\n    title: String,\n    tags: Vec<String>,\n}\n\nimpl Gem {\n    fn new(title: &str) -> Gem {\n        Gem { title: String::from(title), tags: Vec::new() }\n    }\n    \n    fn add_tag(&mut self, tag: &str) {\n        self.tags.push(String::from(tag));\n    }\n    \n    fn is_tagged(&self, tag: &str) -> bool {\n        self.tags.contains(&String::from(tag))\n    }\n}',
          hint: 'The method takes &self (just reading) and returns a bool. Use self.tags.contains(&String::from(tag)) to check.',
          validator: (input: string) => {
            return input.includes('fn is_tagged') && input.includes('&self') && input.includes('-> bool')
          },
        },
      ],
    },

    // ── 6. Error Handling — What If Things Go Wrong? ──
    {
      id: 'error-handling',
      title: 'Step 6: When Things Go Wrong',
      content: [
        {
          type: 'text',
          body: 'Your gem notebook needs to save to a file. But what if the file can\'t be created? What if the disk is full? Most languages deal with this using "exceptions" — surprise problems that blow up your program if you forget to handle them. Rust takes a different approach: it makes you handle problems BEFORE they happen.',
        },
        {
          type: 'text',
          body: 'Imagine ordering food at a restaurant. In most languages, you say "bring me pasta" and HOPE it works — if the kitchen is out of pasta, the waiter screams and flips the table (that\'s an exception). In Rust, the waiter hands you a box that either contains your pasta OR a note saying "sorry, out of pasta." You MUST open the box and check before eating.',
        },
        {
          type: 'concept-card',
          term: 'Result<T, E>',
          explanation: 'A box with two possible contents: Ok(value) if things worked, or Err(problem) if they didn\'t. You MUST check which one you got — the compiler won\'t let you ignore it.',
          example: '// This might fail — returns Result\nlet file = std::fs::read_to_string("gems.txt");\n// file is Ok("contents...") or Err(an error)',
        },
        {
          type: 'concept-card',
          term: 'Option<T>',
          explanation: 'Like Result but simpler: either Some(value) or None. Used when something might not exist — like searching for a gem that might not be in your notebook.',
          example: '// Looking for a gem — might not exist\nlet found: Option<&Gem> = gems.iter()\n    .find(|g| g.title == "Rust notes");\n// found is Some(gem) or None',
        },
        {
          type: 'code',
          language: 'rust',
          code: 'use std::fs;\n\nfn main() {\n    // Try to read a file — this might fail!\n    let result = fs::read_to_string("gems.txt");\n    \n    // You MUST handle both cases\n    match result {\n        Ok(contents) => {\n            println!("File contents: {}", contents);\n        }\n        Err(error) => {\n            println!("Couldn\'t read file: {}", error);\n        }\n    }\n}',
          caption: '`match` forces you to handle success AND failure. The compiler won\'t let you skip either case.',
        },
        {
          type: 'text',
          body: 'Writing `match` every time is verbose. Rust gives you a shortcut: the `?` operator. It means "if this failed, pass the error up to whoever called me — let them deal with it." It\'s like saying "if the kitchen is out of pasta, tell the customer I couldn\'t make their order."',
        },
        {
          type: 'code',
          language: 'rust',
          code: 'use std::fs;\n\n// This function MIGHT fail — notice the Result return type\nfn load_gems() -> Result<String, std::io::Error> {\n    let contents = fs::read_to_string("gems.txt")?;  // ? = pass error up\n    Ok(contents)\n}\n\nfn main() {\n    match load_gems() {\n        Ok(data) => println!("Loaded: {}", data),\n        Err(e) => println!("Failed to load: {}", e),\n    }\n}',
          caption: 'The ? after read_to_string means: if it fails, return the error immediately. No need for a match here.',
        },
        {
          type: 'quiz',
          question: 'Why is Rust\'s Result<T, E> better than exceptions (try/catch)?',
          options: [
            'It\'s faster',
            'The compiler FORCES you to handle errors — you can\'t accidentally forget',
            'It uses less memory',
            'It makes error messages prettier',
          ],
          correctIndex: 1,
          explanation: 'With exceptions (try/catch), you can forget to handle an error — it\'ll crash at runtime when a user hits it. With Result, the compiler refuses to build your program until you handle every possible failure. Bugs caught at compile time can\'t hurt your users.',
        },
        {
          type: 'interactive-code',
          language: 'rust',
          starterCode: '// Complete the function: look up a gem by title\n// If found, return Some(gem_title)\n// If not found, return None\n\nfn find_gem(gems: &Vec<String>, title: &str) -> Option<String> {\n    // Hint: use a for loop or .iter().find()\n}',
          solution: 'fn find_gem(gems: &Vec<String>, title: &str) -> Option<String> {\n    for gem in gems {\n        if gem == title {\n            return Some(gem.clone());\n        }\n    }\n    None\n}',
          hint: 'Loop through the gems. If you find a match, return Some(gem.clone()). After the loop, return None.',
          validator: (input: string) => {
            return input.includes('Some(') && input.includes('None') && input.includes('Option<')
          },
        },
      ],
    },

    // ── 7. Traits — Shared Abilities ──
    {
      id: 'traits',
      title: 'Step 7: Traits — Shared Abilities',
      content: [
        {
          type: 'text',
          body: 'Our gem notebook is growing. Now imagine we want different KINDS of gems: text notes, audio recordings, web links. They\'re all different, but they all need a "summarize" ability. How do we say "all of these must be able to summarize themselves"?',
        },
        {
          type: 'text',
          body: 'Think of a trait like a job requirement. A job posting says: "Must be able to drive, speak English, and use Excel." It doesn\'t care WHO you are — a college student, a retiree, a robot. As long as you can do those things, you qualify. A trait says: "Any type that wants to be a Gem must be able to summarize itself and count its tags."',
        },
        {
          type: 'code',
          language: 'rust',
          code: '// The JOB REQUIREMENTS: what must every gem type be able to do?\ntrait Summarizable {\n    fn summarize(&self) -> String;\n    fn tag_count(&self) -> usize;\n}\n\n// A text gem\nstruct TextGem {\n    title: String,\n    body: String,\n    tags: Vec<String>,\n}\n\n// An audio gem\nstruct AudioGem {\n    title: String,\n    duration_secs: u32,\n    transcript: String,\n    tags: Vec<String>,\n}\n\n// Both FULFILL the requirements differently:\nimpl Summarizable for TextGem {\n    fn summarize(&self) -> String {\n        format!("{}: {}...", self.title, &self.body[..50.min(self.body.len())])\n    }\n    fn tag_count(&self) -> usize { self.tags.len() }\n}\n\nimpl Summarizable for AudioGem {\n    fn summarize(&self) -> String {\n        format!("{}: {}s audio recording", self.title, self.duration_secs)\n    }\n    fn tag_count(&self) -> usize { self.tags.len() }\n}',
          caption: 'Two different types, one shared contract. You can call .summarize() on either one.',
        },
        {
          type: 'text',
          body: 'This is EXACTLY how Jarvis works. It has an "IntelProvider" trait — the job requirements for any AI backend. OpenAI and Apple\'s on-device AI are completely different, but they both fulfill the same contract: summarize text, generate tags. Jarvis doesn\'t care which one you use. It just calls .summarize() and the right implementation runs.',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'trait', label: 'trait IntelProvider', icon: '📋' },
            { id: 'openai', label: 'OpenAI (cloud)', icon: '☁️' },
            { id: 'apple', label: 'Apple on-device', icon: '🍎' },
          ],
          connections: [
            { from: 'trait', to: 'openai', label: 'implements' },
            { from: 'trait', to: 'apple', label: 'implements' },
          ],
        },
        {
          type: 'quiz',
          question: 'What\'s the benefit of using a trait like IntelProvider instead of just writing separate functions?',
          options: [
            'Traits are faster',
            'You can swap implementations without changing the code that uses them',
            'Traits use less memory',
            'Rust requires all functions to be in traits',
          ],
          correctIndex: 1,
          explanation: 'Jarvis can switch from OpenAI to Apple\'s on-device AI by changing ONE line of configuration. All the code that calls .summarize() and .tag() doesn\'t change at all — it just works with whoever fulfills the trait contract.',
        },
        {
          type: 'interactive-code',
          language: 'rust',
          starterCode: '// Define a trait called `Displayable` with one method:\n// fn display(&self) -> String\n// Then implement it for this struct:\n\nstruct Note {\n    title: String,\n    body: String,\n}\n\n// Define the trait and implement it here\n// display should return: "Note: [title] - [body]"',
          solution: 'trait Displayable {\n    fn display(&self) -> String;\n}\n\nstruct Note {\n    title: String,\n    body: String,\n}\n\nimpl Displayable for Note {\n    fn display(&self) -> String {\n        format!("Note: {} - {}", self.title, self.body)\n    }\n}',
          hint: 'First write `trait Displayable { fn display(&self) -> String; }` — that\'s the contract. Then write `impl Displayable for Note { ... }` to fulfill it.',
          validator: (input: string) => {
            return input.includes('trait Displayable') &&
              input.includes('impl Displayable for Note') &&
              input.includes('fn display')
          },
        },
      ],
    },

    // ── 8. Async — Doing Many Things at Once ──
    {
      id: 'async',
      title: 'Step 8: Async — Waiting Without Freezing',
      content: [
        {
          type: 'text',
          body: 'Our gem notebook now needs to talk to the internet — saving to a cloud backup, or calling an AI to summarize. But network requests are SLOW (100ms-2s). If your program just... stops... and waits... the user stares at a frozen screen. That\'s terrible.',
        },
        {
          type: 'text',
          body: 'Think about ordering coffee. You walk up, order, and then... do you stand at the counter staring at the barista until your coffee is ready? No! You sit down, check your phone, maybe chat with a friend. When the coffee is ready, the barista calls your name. That\'s async programming.',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'order', label: 'Start request', icon: '📡' },
            { id: 'wait', label: 'Do other work', icon: '💻' },
            { id: 'ready', label: 'Response arrives', icon: '📬' },
            { id: 'use', label: 'Use the result', icon: '✅' },
          ],
          connections: [
            { from: 'order', to: 'wait', label: 'don\'t freeze' },
            { from: 'wait', to: 'ready', label: '.await wakes up' },
            { from: 'ready', to: 'use' },
          ],
        },
        {
          type: 'code',
          language: 'rust',
          code: '// Without async: FROZEN while waiting\nfn save_to_cloud(gem: &Gem) -> Result<(), Error> {\n    let response = http_client.post(url).send(); // BLOCKED! Nothing else can happen\n    // ... user is staring at a frozen app for 2 seconds\n}\n\n// With async: keep working while waiting\nasync fn save_to_cloud(gem: &Gem) -> Result<(), Error> {\n    let response = http_client.post(url).send().await; // go do other stuff, come back when ready\n    // ... app stays responsive the whole time\n}',
          caption: '`.await` means "start this, and come back to me when it\'s done — in the meantime, handle other tasks"',
        },
        {
          type: 'text',
          body: 'Jarvis uses async EVERYWHERE. When you hit the record button, it simultaneously: captures audio, monitors the browser, updates the UI, and keeps the app responsive. Without async, each of these would have to wait for the others to finish. With async, they all make progress together.',
        },
        {
          type: 'concept-card',
          term: 'async fn',
          explanation: 'Marks a function as "this might need to wait for something slow." The function can pause and resume without blocking everything else.',
          example: 'async fn summarize(text: &str) -> String {\n    let result = ai.process(text).await;\n    result\n}',
        },
        {
          type: 'concept-card',
          term: '.await',
          explanation: '"Start this task and come back when it\'s done." Like placing your coffee order and sitting down. You\'ll be notified when it\'s ready.',
          example: 'let summary = summarize(text).await;\n// While waiting, other async tasks ran',
        },
        {
          type: 'concept-card',
          term: 'Tokio',
          explanation: 'The "coffee shop manager" — it schedules all the async tasks, wakes them up when their data arrives, and makes sure nothing gets stuck. Jarvis uses Tokio as its async runtime.',
          example: '#[tokio::main]\nasync fn main() {\n    // Now you can use .await in main\n}',
        },
        {
          type: 'quiz',
          question: 'In Jarvis, when you click "Record", what would happen WITHOUT async?',
          options: [
            'Recording would be faster',
            'The app would freeze while capturing audio — no UI updates, no cancel button, nothing until recording finishes',
            'Multiple recordings would start at once',
            'The recording would save automatically',
          ],
          correctIndex: 1,
          explanation: 'Without async, the app would block on audio capture. The UI would freeze — you couldn\'t see a timer, couldn\'t click "Stop", couldn\'t even close the window. Async lets Jarvis capture audio, update the UI, and listen for your stop click all at the same time.',
        },
      ],
    },

    // ── 9. Modules — Organizing Your Growing Project ──
    {
      id: 'modules',
      title: 'Step 9: Modules — Organizing Your Code',
      content: [
        {
          type: 'text',
          body: 'Your gem notebook has grown: structs, traits, error handling, async functions. Having everything in one file is like keeping every document you own in a single drawer. Eventually you can\'t find anything. Modules are Rust\'s filing system.',
        },
        {
          type: 'text',
          body: 'Think of your house. You don\'t keep your toothbrush in the kitchen. You have rooms: kitchen for cooking, bathroom for hygiene, bedroom for sleeping. Modules are rooms for your code.',
        },
        {
          type: 'code',
          language: 'bash',
          code: '# Your project structure\ngem-notebook/\n  src/\n    main.rs          # Front door — the entry point\n    gems/\n      mod.rs          # Room directory — "gems room contains..."\n      types.rs        # Gem struct definition\n      store.rs        # Save/load gems to file\n    intelligence/\n      mod.rs          # "intelligence room contains..."\n      summarize.rs    # AI summarization logic',
          caption: 'Each folder is a module. mod.rs is the "table of contents" for that module.',
        },
        {
          type: 'code',
          language: 'rust',
          code: '// src/main.rs — the front door\nmod gems;          // "I have a gems room" → loads gems/mod.rs\nmod intelligence;  // "I have an intelligence room"\n\nuse gems::Gem;     // "Bring the Gem type into this room"\nuse intelligence::summarize;\n\nfn main() {\n    let gem = Gem::new("Notes", "Some content");\n    let summary = summarize(&gem.content);\n    println!("{}", summary);\n}',
          caption: '`mod` declares a room exists. `use` brings items from that room into the current scope.',
        },
        {
          type: 'text',
          body: 'Jarvis uses this exact structure. Its backend has rooms for: commands (what the frontend can ask for), gems (storage and search), intelligence (AI summaries), browser (web extraction), and recording (audio capture). Each room is self-contained but they can share items using `pub` (public — visible to other rooms).',
        },
        {
          type: 'diagram',
          nodes: [
            { id: 'lib', label: 'lib.rs (front door)', icon: '🏠' },
            { id: 'cmd', label: 'commands/', icon: '🎮' },
            { id: 'gems', label: 'gems/', icon: '💎' },
            { id: 'intel', label: 'intelligence/', icon: '🧠' },
            { id: 'browser', label: 'browser/', icon: '🌐' },
            { id: 'rec', label: 'recording/', icon: '🎙️' },
          ],
          connections: [
            { from: 'lib', to: 'cmd', label: 'mod' },
            { from: 'lib', to: 'gems', label: 'mod' },
            { from: 'lib', to: 'intel', label: 'mod' },
            { from: 'lib', to: 'browser', label: 'mod' },
            { from: 'lib', to: 'rec', label: 'mod' },
          ],
        },
        {
          type: 'concept-card',
          term: 'mod',
          explanation: '"This room exists." Tells Rust to look for the module\'s code in a matching file or folder.',
          example: 'mod gems;      // loads gems.rs or gems/mod.rs\nmod recording; // loads recording.rs',
        },
        {
          type: 'concept-card',
          term: 'use',
          explanation: '"Bring this item into my room." Like carrying a tool from the garage to the kitchen.',
          example: 'use crate::gems::Gem;\nuse crate::intelligence::IntelProvider;',
        },
        {
          type: 'concept-card',
          term: 'pub',
          explanation: '"This is visible outside my room." Without pub, items are private — only code in the same module can see them.',
          example: 'pub struct Gem { ... }   // others can see\nstruct InternalHelper { ... } // private',
        },
        {
          type: 'quiz',
          question: 'Why is `pub` (public) important?',
          options: [
            'It makes the code run faster',
            'Without it, nothing in one module can be used by another module — everything is private by default',
            'It\'s required for all structs',
            'It publishes the code to the internet',
          ],
          correctIndex: 1,
          explanation: 'Rust makes everything private by default. Only items marked `pub` can be used from other modules. This is intentional — it forces you to design clean boundaries between parts of your code, exposing only what\'s needed.',
        },
      ],
    },
  ],

  jarvisConnections: [
    {
      concept: 'Structs & Ownership',
      file: 'src-tauri/src/lib.rs',
      description: 'The AppState struct holds all of Jarvis\'s shared data (gem store, recording state, AI provider). It uses ownership concepts like Arc and Mutex to safely share across the app.',
    },
    {
      concept: 'Traits for Swappable AI',
      file: 'src-tauri/src/intelligence/provider.rs',
      description: 'The IntelProvider trait defines what any AI backend must do. OpenAI and Apple\'s IntelligenceKit both implement it — Jarvis doesn\'t care which one is active.',
    },
    {
      concept: 'Error Handling with Result',
      file: 'src-tauri/src/commands.rs',
      description: 'Every command returns Result. If anything fails (file not found, network error, sidecar crash), the error flows back to the frontend as a clear message.',
    },
    {
      concept: 'Async Everywhere',
      file: 'src-tauri/src/recording.rs',
      description: 'Recording uses async to simultaneously: spawn the audio sidecar, read its output, monitor for errors, and emit events to the UI — all without freezing.',
    },
    {
      concept: 'Module Organization',
      file: 'src-tauri/src/gems/sqlite_store.rs',
      description: 'The gems module is split into types, storage, and search. Each file handles one concern. The module\'s mod.rs re-exports the public items.',
    },
  ],
}
