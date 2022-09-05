 
 export OPENSSL_LIB_DIR=$HOME/openssl/lib
export OPENSSL_INCLUDE_DIR=$HOME/openssl/include
 export OPENSSL_STATIC=true

 export PATH="$HOME/aarch64-linux-musl-native/bin:$PATH"
 export PATH="$HOME/aarch64-linux-musl-native/aarch64-linux-musl/bin:$PATH"
          cargo build --target aarch64-unknown-linux-musl --release