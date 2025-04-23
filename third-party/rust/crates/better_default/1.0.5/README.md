# Better Default

*The std Default derive with more customization available and some upgrades.*

[<img alt="Static Badge" src="https://img.shields.io/badge/github-better_default-blue">](https://github.com/NovaliX-Dev/better_default)
[<img alt="Crates.io Version" src="https://img.shields.io/crates/v/better_default?color=red">](https://crates.io/crates/better_default)
[<img alt="docs.rs" src="https://img.shields.io/docsrs/better_default">](https://docs.rs/better_default/)
![Crates.io License](https://img.shields.io/crates/l/better_default)


This crate provide a single derive trait called `Default`. This derive act as the std `Default` derive, but allows to modify the default values of each fields. It also allows to mark enum variants with fields as default.

## Features
 - Does everything the std `Default` derive trait does
 - Support marking enum variant with fields as default
 - Support overriding the default value of each fields
 - Support no-std, which means it will output code which is no-std. **Note that this library by itself needs the std library**.

See all those features in actions in the `Examples` chapter.

## How to use

> **Before doing anything here**, if you want to override the fields of an enum variant, **you should mark it as default first**

```rust, ignore
use better_default::Default;

#[derive(Default)]
enum Enum {
    #[default]
    Variant {
        ...
    },
    ...
}
```

### 1. Overriding the default values

There a two ways of overriding the default values : using the per-field attributes or the top default attributes.

#### Per-Field attributes

The per-field attributes are simply attributes you put atop of the fields for which you want to override the default values.

The syntax is the following :
```rust, ignore
#[default( <expression> )]
<field_ident>: <field_type>
```

You can put anything you want in the `expression` bloc, **as long as it can be correctly parse by [syn::Expr](https://docs.rs/syn/latest/syn/enum.Expr.html).**

Here is an example of this approach in action :
```rust
use better_default::Default;

#[derive(Default, Debug)]
enum Enum {
    #[default] // mark the variant as default (this is mandatory)
    Variant1 {
        #[default(1)] // set the default value of `first` to 1
        first: u32,

        // keep the default value for `second`
        second: String,
    },

    Variant2,

    Variant3,
}

fn main() {
    let default = Enum::default();
    
    // should print "Variant1 { first: 1, second: "" }"
    println!("{:?}", default);
}
```

#### Top Default attributes

Instead of placing an attribute on all the fields of a struct / enum variant, you place only on attribute atop of it, containing all the default values overrides.

The syntax of the top attributes is the following :
```rust, ignore
use better_default::Default;

#[derive(Default)]
#[default((<field_id>: <expression>),*)]
struct Struct { 
    ...
} // the struct can have unnamed fields

#[derive(Default)]
enum Enum {
    #[default((<field_id>: <expression>),*)]
    Variant { ... } // the variant can have unnamed fields
}
```

`field_id` here can means two things : if you deal with named fields, you should put the field ident here. If you deal with unnamed fields, then **you should put the position of the field** *(0 for the first, 1 for the second, etc.)*.

Again, you can put anything you want in the `expression` bloc, **as long as it can be correctly parse by [syn::Expr](https://docs.rs/syn/latest/syn/enum.Expr.html).**

Here are two examples, one covering unnamed fields and one covering named ones.
```rust
use better_default::Default;

#[derive(Default, Debug)]
enum Enum {
    // mark the variant as default, and also specifies the default values :
    //      - the default value of the first field (which is at index 0) is set to 2
    //      - the second field (which is at index 1) will have it's default value set to "Hello world!"
    #[default(0: 2, 1: "Hello world!".to_string())]
    Variant1(u32, String),

    Variant2,

    Variant3,
}

fn main() {
    let default = Enum::default();
    
    // should print "Variant1(2, "Hello world!")"
    println!("{:?}", default);
}
```

```rust
use better_default::Default;

#[derive(Default, Debug)]
#[default(field1: 1, field2: "Hello world!".to_string())]
struct Struct {
    field1: u32, 
    field2: String
}

fn main() {
    let default = Struct::default();
    println!("{:?}", default) // should print "Struct { field1: 1, field2: "Hello world!" }"
}
```

One last note : **these two approaches can be combined,
which means you can have a top attribute containing some
default values while some of the fields have their
own attribute.**

## Examples

1) **The per-field way : Usage of per-field attributes**

Per field attributes are more suitable for struct / enum variants with named fields.

```rust
use better_default::Default;

#[derive(Default, Debug)]
enum Enum {
    #[default] // mark the variant as default (this is mandatory)
    Variant1 {
        #[default(1)] // set the default value of `first` to 1
        first: u32,

        // keep the default value for `second`
        second: String,
    },

    Variant2,

    Variant3,
}

fn main() {
    let default = Enum::default();
    
    // should print "Variant1 { first: 1, second: "" }"
    println!("{:?}", default);
}
```

```rust
use better_default::Default;

#[derive(Default, Debug)]
// Structs don't need to be mark as default with a top attribute. They're optional.
struct Struct {
    #[default(10)] // set the default value of field1 to be 10
    field1: u32,

    // keeps the usual default value for field2
    field2: String,
}

fn main() {
    let default = Struct::default();
    println!("{:?}", default) // should print "Struct { field1: 10, field2: "" }"
}

```

While not recommended, you can also use them on unnamed fields :

```rust
use better_default::Default;

#[derive(Default, Debug)]
// Structs don't need to be mark as default with a top attribute. They're optional.
struct Struct (
    #[default(10)] // set the default value of field1 to be 10
    u32,

    // keeps the usual default value for field2
    String,
);

fn main() {
    let default = Struct::default();
    println!("{:?}", default) // should print "Struct(10, "")"
}
```

```rust
use better_default::Default;

#[derive(Default, Debug)]
enum Enum {
    #[default] // mark the variant as default (this is mandatory)
    Variant1 (
        #[default(1)] // set the default value to 1
        u32,

        // keep the default value
        String,
    ),

    Variant2,

    Variant3,
}

fn main() {
    let default = Enum::default();
    
    // should print "Variant1(1, "")"
    println!("{:?}", default);
}
```

2) **The all at once way : Usage of top default attributes**

The particularity of the top attribute is that you can define all the default values at the same place.

> **Not all the fields need to be represented here, only those you want to modify.**

```rust
use better_default::Default;

#[derive(Default, Debug)]
enum Enum {
    // mark the variant as default, and also specifies the default values :
    //      - the first field keeps it's usual default value.
    //      - the second field (which is at index 1) will have it's default value set to "Hello world!"
    #[default(1: "Hello world!".to_string())]
    Variant1(u32, String),

    Variant2,

    Variant3,
}

fn main() {
    let default = Enum::default();
    
    // should print "Variant1(0, "Hello world!")"
    println!("{:?}", default);
}
```

```rust
use better_default::Default;

#[derive(Default, Debug)]
// here we can use the top default attribute to customize the default values of our fields.
//      - we change the default value of the first field (represented by the index 0) to 1
#[default(0: 1, 1: "a".to_string())]
struct Struct(u32, String);

fn main() {
    let default = Struct::default();
    println!("{:?}", default) // should print "Struct(1, "a")"
}
```

This can also be used on named fields :

```rust
use better_default::Default;

#[derive(Default, Debug)]
enum Enum {
    // mark the variant as default, and also specifies the default values :
    //      - the first field keeps it's usual default value.
    //      - the second field (field2) will have it's default value set to "Hello world!"
    #[default(field2: "Hello world!".to_string())]
    Variant1 {
        field1: u32, 
        field2: String
    },

    Variant2,

    Variant3,
}

fn main() {
    let default = Enum::default();
    
    // should print "Variant1 { 0, "Hello world!" }"
    println!("{:?}", default);
}
```

```rust
use better_default::Default;

#[derive(Default, Debug)]
#[default(field1: 1, field2: "Hello world!".to_string())]
struct Struct {
    field1: u32, 
    field2: String
}

fn main() {
    let default = Struct::default();
    println!("{:?}", default) // should print "Struct { field1: 1, field2: "Hello world!" }"
}
```

## Contributing

You can contribute to the project by making a pull request.

Here are the tools i use for this library :

- [rustdoc-include](https://github.com/frozenlib/rustdoc-include), which allows me to import the readme directly into the `lib.rs` without copying. That's why you can see those `// #[include_doc(...)]` in `lib.rs`. Use the `build_crate_doc` script in the `scripts` folder to update them.

## License

Licensed under Apache 2.0.