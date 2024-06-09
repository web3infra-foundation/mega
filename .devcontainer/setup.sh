## update and install some things we should probably have
apt-get update
apt-get install -y \
  curl \
  git \
  gnupg2 \
  jq \
  sudo \
  zsh \
  vim \
  build-essential \
  openssl \
  libssl-dev \
  fuse3 \
  libfuse3-dev \
  pkg-config \
  postgresql

## Install rustup and common components
curl https://sh.rustup.rs -sSf | sh -s -- -y 
rustup install default
rustup component add rustfmt
rustup component add clippy

cargo install cargo-expand
cargo install cargo-edit

## Setup and install oh-my-zsh
sh -c "$(curl -fsSL https://raw.githubusercontent.com/robbyrussell/oh-my-zsh/master/tools/install.sh)"
cp -R /root/.oh-my-zsh /home/$USERNAME
cp /root/.zshrc /home/$USERNAME
sed -i -e "s/\/root\/.oh-my-zsh/\/home\/$USERNAME\/.oh-my-zsh/g" /home/$USERNAME/.zshrc
chown -R $USER_UID:$USER_GID /home/$USERNAME/.oh-my-zsh /home/$USERNAME/.zshrc

## Start PostgreSQL
/etc/init.d/postgresql start

sudo -u postgres psql mega -c "CREATE DATABASE mega;"
sudo -u postgres psql mega < /workspaces/mega/sql/pg_20240205__init.sql
sudo -u postgres psql mega -c "CREATE USER mega WITH ENCRYPTED PASSWORD 'mega';"
sudo -u postgres psql mega -c "GRANT ALL PRIVILEGES ON DATABASE mega TO mega;"
sudo -u postgres psql mega -c "GRANT ALL ON ALL TABLES IN SCHEMA public to mega;"
sudo -u postgres psql mega -c "GRANT ALL ON ALL TABLES IN SCHEMA public to mega;"
sudo -u postgres psql mega -c "GRANT ALL ON ALL SEQUENCES IN SCHEMA public to mega;"
sudo -u postgres psql mega -c "GRANT ALL ON ALL FUNCTIONS IN SCHEMA public to mega;"