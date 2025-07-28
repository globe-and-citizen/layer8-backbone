dev-docker:
	cp -R ../certs certs
	cp -R ../server_conf.yml server_conf.yml
	cp docker.dev.env .env
	docker build --progress=plain -t layer8-forward-proxy:dev .
	rm -fr certs server_conf.yml .env 

run-fp-docker: dev-docker
	docker run -p 6191:6191 -t layer8-forward-proxy:dev