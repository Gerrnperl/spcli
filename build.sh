mkdir temp
cp -R ./text ./temp/text
cp -R ./keymap ./temp/keymap
echo "Compling for Linux"
cargo build --release
cp ./target/release/spcli ./temp/spcli
tar czf spcli-linux-x86_64.tar.gz --directory="./temp" .
rm ./temp/spcli
echo "Compling for Windows"
cargo build --release --target x86_64-pc-windows-gnu
cp ./target/x86_64-pc-windows-gnu/release/spcli.exe ./temp/spcli.exe
tar czf spcli-windows-x86_64.tar.gz --directory="./temp" .
rm -rf ./temp