SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd $SCRIPT_DIR && cd ..
SRC_DIR=$(pwd)
cd $SRC_DIR

cargo publish --dry-run -p freenet-macros || { exit 1; }
read -p "Publish freenet-macros? " -n 1 -r
echo   
if [[ $REPLY =~ ^[Yy]$ ]]
then
    cargo publish -p freenet-macros
else 
	echo "Not publishing freenet-macros"
fi

cargo publish --dry-run -p freenet-stdlib || { exit 1; }
read -p "Publish freenet-stdlib? " -n 1 -r
echo   
if [[ $REPLY =~ ^[Yy]$ ]]
then
    cargo publish -p freenet-stdlib
else 
	echo "Not publishing freenet-stdlib"
fi
