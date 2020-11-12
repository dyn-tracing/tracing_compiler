# start minikube
minikube start

# this injects istio into the cluster
deps/istio-1.7.4/bin/istioctl install
kubectl label namespace default istio-injection=enabled

# set the wasme path in the fish shell
cd wasm
./install_cli.sh
set -gx PATH $HOME/.wasme/bin:$PATH $PATH

# deploy the microservices-demo
# this command might have to be run multiple times because the demo sucks
cd deps/microservices-demo; skaffold run; cd -

# build our filter, replace this with our custom query stuff later
wasme build assemblyscript -t test-filter . \
                                   --tag webassemblyhub.io/fruffy/test-filter:1  \
                                   --config runtime-config.json


wasme deploy istio webassemblyhub.io/fruffy/test-filter:1 –provider=istio --id test

# launch the frontend

minikube service frontend-external
export INGRESS_PORT=(kubectl get service frontend-external -o jsonpath='{.spec.ports[*].nodePort}')
export INGRESS_HOST=(minikube ip)
export GATEWAY_URL=$INGRESS_HOST:$INGRESS_PORT
# get the http headers of the front page
curl -v "http://$GATEWAY_URL"


# The wasm runtime-config should look similiar to this
# {
#   "type": "envoy_proxy",
#   "abiVersions": [
#     "v0-4689a30309abf31aee9ae36e73d34b1bb182685f", <--- newer version of istio
#     "v0-541b2c1155fffb15ccde92b8324f3e38f7339ba6",
#     "v0-097b7f2e4cc1fb490cc1943d0d633655ac3c522f",
#     "v0.2.1"
#   ],
#   "config": {
#     "rootIds": [
#       "add_header"
#     ]
#   }
# }