# docker build -f scripts/engine.Dockerfile -o engine .
FROM emscripten/emsdk:6.0.13@sha256:fb7864e398acfda863dfcea78b8bc8adad359fa60c6a0bbeb845931de64aa68a AS build
WORKDIR /src
COPY sources/hunspell-1.7.5.tar.gz sources/
COPY scripts/build-engine.sh scripts/
RUN bash scripts/build-engine.sh

FROM scratch
COPY --from=build /src/engine/ /
