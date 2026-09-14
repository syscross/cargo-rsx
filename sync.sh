
git add .

msg=""

if [ $# -eq 0 ]; then
    msg="sync"
else
    msg="$@"
fi

git commit -am "$msg"

git pull

git push
