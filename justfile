run arg="":
    cargo run -- {{arg}}

add event="":
    cargo run -- add {{event}}

did event="":
    cargo run -- did {{event}}

remove event="":
    cargo run -- remove {{event}}

check event="":
    cargo run -- {{event}}

list:
    cargo run -- list
