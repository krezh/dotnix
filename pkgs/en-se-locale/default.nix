{ glibcLocales }:

glibcLocales.overrideAttrs (oldAttrs: {
  pname = "glibc-locales-en-se";
  postPatch = (oldAttrs.postPatch or "") + ''
    cp ${./en_SE} localedata/locales/en_SE
    echo 'en_SE.UTF-8/UTF-8 \' >> localedata/SUPPORTED
  '';
})
