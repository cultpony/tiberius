curl -sS https://dl.yarnpkg.com/debian/pubkey.gpg | apt-key add -
echo "deb https://dl.yarnpkg.com/debian/ stable main" | tee /etc/apt/sources.list.d/yarn.list
apt update
apt install --assume-yes --no-install-recommends yarn sudo
adduser --uid 1000 --disabled-password --gecos "" user
cd /app

echo "===== BEGIN DEPLOY BUILD ====="

sudo -u user curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
sudo -u user nvm install 16
sudo -u user nvm use 16
sudo -u user cargo install just
sudo -u user just build-release