/// GET the env value from var str
pub fn get_env_number<T:std::str::FromStr>(var_name:&str, value :&mut T) {
    if let Ok(env_value)  = std::env::var(var_name){
        if let Ok(true_value) = env_value.parse::<T>(){
            *value = true_value;
        }
    }
}