/* --------------------------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 * ------------------------------------------------------------------------------------------ */

import {
  workspace,
  type ExtensionContext,
  window,
  commands
} from 'vscode'

import {
  type Executable,
  LanguageClient,
  type LanguageClientOptions,
  type ServerOptions
} from 'vscode-languageclient/node'

let client: LanguageClient

export async function activate (context: ExtensionContext): Promise<void> {
  const disposable = commands.registerCommand('helloworld.helloWorld', async uri => {
    // The code you place here will be executed every time your command is executed
    // https://code.visualstudio.com/api/extension-guides/command#registering-a-command
    console.log('Running command registered as helloworld.helloWorld')
    console.log(uri)
  })

  context.subscriptions.push(disposable)

  const traceOutputChannel = window.createOutputChannel('Generic Language Server trace')

  const command = process.env.SERVER_PATH ?? 'generic-language-server'
  console.log(command)

  const run: Executable = {
    command,
    options: {
      env: {
        ...process.env,
        // eslint-disable-next-line @typescript-eslint/naming-convention
        RUST_LOG: 'debug'
      }
    }
  }

  // If the extension is launched in debug mode then the debug server options are used
  // Otherwise the run options are used
  const serverOptions: ServerOptions = {
    run,
    debug: run
  }

  // Options to control the language client
  const clientOptions: LanguageClientOptions = {
    // Register the server for plain text documents
    documentSelector: [{ scheme: 'file', language: 'gen' }],
    synchronize: {
      // Notify the server about file changes to '.gen' files contained in the workspace
      // https://vscode-api.js.org/modules/vscode.workspace.html#createFileSystemWatcher
      fileEvents: workspace.createFileSystemWatcher('**/*.gen')
    },
    traceOutputChannel
  }

  // Create the language client and start the client.
  client = new LanguageClient(
    'generic-language-server',
    'Generic language server',
    serverOptions,
    clientOptions
  )

  console.log('Running Generic LSP extention')
  await client.start()
}

export function deactivate (): Thenable<void> | undefined {
  console.log('Exiting Generic LSP extention')
  if (client === null) {
    return undefined
  }
  return client.stop()
}
