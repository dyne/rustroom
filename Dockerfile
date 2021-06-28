#
# Build this image with the command
#   docker build -f docker/build -t dyne/rust:latest
#
# Then run with the command
#   docker run -it dyne/rust:latest
#

FROM dyne/rust:beowulf
LABEL maintainer="Denis Roio <jaromil@dyne.org>" \
	  homepage="https://github.com/dyne/rust"
# ENV VERSION=AUTO_STRICT

WORKDIR /app

RUN git clone https://github.com/dyne/rustroom \
	&& cd rustroom && cargo build --release

CMD /bin/bash
