use crate::TokenStream2;
use quote::ToTokens;

pub trait JoinTokens {
    fn join_tokens<S: ToTokens>(&self, separator: &S) -> TokenStream2;
}

impl<T: ToTokens> JoinTokens for Vec<T> {
    fn join_tokens<S: ToTokens>(&self, separator: &S) -> TokenStream2 {
        let mut tokens = TokenStream2::new();

        if self.is_empty() {
            return tokens;
        }

        let mut iter = self.iter();
        iter.next().to_tokens(&mut tokens);

        for el in iter {
            separator.to_tokens(&mut tokens);
            el.to_tokens(&mut tokens);
        }

        tokens
    }
}
