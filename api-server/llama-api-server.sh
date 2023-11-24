#!/bin/bash

# Initialize our own variables
update_model=false
all=false
update_server_app=false
restart_server=false

# Parse the command-line arguments
while (( "$#" )); do
  case "$1" in
    --update-model)
      if [ -n "$2" ] && [ ${2:0:1} != "-" ]; then
        update_model=$2
        shift 2
      else
        echo "Error: Argument for $1 is missing" >&2
        exit 1
      fi
      ;;
    --all)
      if [ -n "$2" ] && [ ${2:0:1} != "-" ]; then
        all=$2
        shift 2
      else
        echo "Error: Argument for $1 is missing" >&2
        exit 1
      fi
      ;;
    --update-server-app)
      if [ -n "$2" ] && [ ${2:0:1} != "-" ]; then
        update_server_app=$2
        shift 2
      else
        echo "Error: Argument for $1 is missing" >&2
        exit 1
      fi
      ;;
    --restart-server)
      if [ -n "$2" ] && [ ${2:0:1} != "-" ]; then
        restart_server=$2
        shift 2
      else
        echo "Error: Argument for $1 is missing" >&2
        exit 1
      fi
      ;;
    -*|--*=) # unsupported flags
      echo "Error: Unsupported flag $1" >&2
      exit 1
      ;;
    *) # preserve positional arguments
      PARAMS="$PARAMS $1"
      shift
      ;;
  esac
done

########### Step 1: Checking the operating system ###########

if [ "$all" = true ]; then

    printf "(1/6) Checking the operating system (macOS and Linux supported) ...\n\n"

    # Check if the current operating system is macOS or Linux
    if [[ "$OSTYPE" != "linux-gnu"* && "$OSTYPE" != "darwin"* ]]; then
        echo "The OS should be macOS or Linux"
        exit 1
    fi

fi

########### Step 2: Checking if git and curl are installed ###########

if [ "$all" = true ]; then

    printf "(2/6) Checking if 'git' and 'curl' are installed ...\n\n"

    # Check if git and curl are installed, if not, install them
    for cmd in git curl
    do
        if ! command -v $cmd &> /dev/null
        then
            if [[ "$OSTYPE" == "linux-gnu"* ]]; then
                sudo apt-get install $cmd
            fi
            printf "\n"

            if [[ "$OSTYPE" == "darwin"* ]]; then
                brew install $cmd
            fi
            printf "\n"
        fi
    done

else
    printf "(2/6) Checking if 'git' and 'curl' are installed ...(ignored)\n\n"
fi

########### Step 3: Installing WasmEdge ###########

if [ "$all" = true ]; then

    printf "(3/6) Installing WasmEdge ...\n\n"

    # Run the command to install WasmEdge
    if curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/install.sh | bash -s -- --plugins wasi_nn-ggml; then
        source $HOME/.wasmedge/env
        wasmedge_path=$(which wasmedge)
        printf "\n      The WasmEdge Runtime is installed in %s.\n\n      * To uninstall it, use the command 'bash <(curl -sSf https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/utils/uninstall.sh) -q'\n" "$wasmedge_path"
    else
        echo "Failed to install WasmEdge"
        exit 1
    fi

    printf "\n"

else
    printf "(3/6) Installing WasmEdge ...(ignored)\n\n"
fi

########### Step 4: Downloading the model ###########

# If --update-model is true or --all is true, run the selected code
if [ "$all" = true ] || [ "$update_model" = true ]; then

    printf "(4/6) Downloading the gguf model ...\n\n"

    models="llama-2-7b-chat https://huggingface.co/second-state/Llama-2-7B-Chat-GGUF/resolve/main/llama-2-7b-chat.Q5_K_M.gguf llama-2-chat \
    llama-2-13b-chat https://huggingface.co/second-state/Llama-2-13B-Chat-GGUF/resolve/main/llama-2-13b-chat.Q5_K_M.gguf llama-2-chat \
    mistrallite https://huggingface.co/second-state/MistralLite-7B-GGUF/resolve/main/mistrallite.Q5_K_M.gguf mistrallite \
    tinyllama-1.1b-chat https://huggingface.co/second-state/TinyLlama-1.1B-Chat-v0.3-GGUF/resolve/main/tinyllama-1.1b-chat-v0.3.Q5_K_M.gguf llama-2-chat"
    model_names="llama-2-7b-chat llama-2-13b-chat mistrallite tinyllama-1.1b-chat"

    # Convert model_names to an array
    model_names_array=($model_names)

    # Print the models with their corresponding numbers
    for i in "${!model_names_array[@]}"; do
    printf "      %d) %s\n" $((i+1)) "${model_names_array[$i]}"
    done

    printf "\n      Please enter a number from the list above: "
    read model_number

    # Validate the input
    while [[ "$model_number" -lt 1 || "$model_number" -gt ${#model_names_array[@]} ]]; do
        printf "\n      Invalid number. Please enter a number between 1 and %d: " ${#model_names_array[@]}
        read model_number
    done

    # Get the model name from the array
    model=${model_names_array[$((model_number-1))]}

    # Change IFS to newline
    IFS=$'\n'

    # Check if the provided model name exists in the models string
    url=$(printf "%s\n" $models | awk -v model=$model '{for(i=1;i<=NF;i++)if($i==model)print $(i+1)}')

    if [ -z "$url" ]; then
        printf "\n      The URL for downloading the target gguf model does not exist.\n"
        exit 1
    fi

    printf "\n      You picked %s, downloading from %s\n" "$model" "$url"
    curl -LO $url -#

    # get the model name from the url
    model_file=$(basename $url)

    # Check if the provided model name exists in the models string
    prompt_template=$(printf "%s\n" $models | awk -v model=$model '{for(i=1;i<=NF;i++)if($i==model)print $(i+2)}')

    if [ -z "$prompt_template" ]; then
        printf "\n      The prompt template for the selected model does not exist.\n"
        exit 1
    fi

    printf "\n"

else
    printf "(4/6) Downloading the gguf model ...(ignored)\n\n"
fi

########### Step 5: Downloading the wasm file ###########

# If --update-server is true or --all is true, perform some action
if [ "$all" = true ] || [ "$update_server_app" = true ]; then

    printf "(5/6) Downloading 'llama-api-server' wasm app ...\n\n"

    wasm_url="https://github.com/second-state/llama-utils/raw/main/api-server/llama-api-server.wasm"
    curl -LO $wasm_url -#

    printf "\n"

else
    printf "(5/6) Downloading 'llama-api-server' wasm app ...(ignored)\n\n"
fi

########### Step 6: Start llama-api-server ###########

if [ "$all" = true ]; then

    printf "(6/6) Starting llama-api-server ...\n\n"

    # Start the server and save the PID
    wasmedge --dir .:. --nn-preload default:GGML:AUTO:$model_file llama-api-server.wasm -p $prompt_template &
    echo $! > wasmedge_pid.txt

fi

if [ "$all" = false ] && ([ "$update_model" = true ] || [ "$update_server_app" = true ] || [ "$restart_server" = true ]); then

    printf "(6/6) Restarting llama-api-server ...\n\n"

    # Get the PID from the file
    wasmedge_pid=$(cat wasmedge_pid.txt)

    # Kill the server
    kill $wasmedge_pid

    # Start the server and save the PID
    wasmedge --dir .:. --nn-preload default:GGML:AUTO:$model_file llama-api-server.wasm -p $prompt_template &
    echo $! > wasmedge_pid.txt

fi