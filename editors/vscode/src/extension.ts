import * as vscode from 'vscode';

let outputChannel: vscode.OutputChannel;

export function activate(context: vscode.ExtensionContext) {
    outputChannel = vscode.window.createOutputChannel('rx');

    // Register all rx commands
    const commands: [string, string][] = [
        ['rx.build', 'build'],
        ['rx.buildRelease', 'build --release'],
        ['rx.test', 'test'],
        ['rx.fmt', 'fmt'],
        ['rx.lint', 'lint'],
        ['rx.check', 'check'],
        ['rx.fix', 'fix'],
        ['rx.ci', 'ci'],
        ['rx.clean', 'clean'],
        ['rx.doctor', 'doctor'],
        ['rx.insights', 'insights'],
        ['rx.deps', 'deps'],
        ['rx.coverage', 'coverage'],
        ['rx.watch', 'watch'],
        ['rx.run', 'run'],
    ];

    for (const [commandId, rxCommand] of commands) {
        const disposable = vscode.commands.registerCommand(commandId, () => {
            runRxCommand(rxCommand);
        });
        context.subscriptions.push(disposable);
    }

    // Auto-check on save
    const config = vscode.workspace.getConfiguration('rx');
    if (config.get<boolean>('autoCheck', true)) {
        const saveWatcher = vscode.workspace.onDidSaveTextDocument((doc) => {
            if (doc.languageId === 'rust' || doc.fileName.endsWith('Cargo.toml')) {
                runRxCommand('check', true);
            }
        });
        context.subscriptions.push(saveWatcher);
    }

    // Task provider
    const taskProvider = vscode.tasks.registerTaskProvider('rx', {
        provideTasks(): vscode.Task[] {
            const tasks: vscode.Task[] = [];
            const rxCommands = ['build', 'test', 'fmt', 'lint', 'check', 'ci', 'clean'];

            for (const cmd of rxCommands) {
                const task = new vscode.Task(
                    { type: 'rx', command: cmd },
                    vscode.TaskScope.Workspace,
                    cmd,
                    'rx',
                    new vscode.ShellExecution(`rx ${cmd}`),
                    '$rustc'
                );
                tasks.push(task);
            }

            return tasks;
        },
        resolveTask(task: vscode.Task): vscode.Task | undefined {
            const command = task.definition.command;
            if (command) {
                const profile = task.definition.profile;
                const profileFlag = profile ? `--profile ${profile} ` : '';
                return new vscode.Task(
                    task.definition,
                    vscode.TaskScope.Workspace,
                    command,
                    'rx',
                    new vscode.ShellExecution(`rx ${profileFlag}${command}`),
                    '$rustc'
                );
            }
            return undefined;
        }
    });
    context.subscriptions.push(taskProvider);

    // Status bar
    const statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    statusBar.text = '$(tools) rx';
    statusBar.tooltip = 'rx - Rust Toolchain Manager';
    statusBar.command = 'rx.build';
    statusBar.show();
    context.subscriptions.push(statusBar);

    outputChannel.appendLine('rx extension activated');
}

function runRxCommand(command: string, silent: boolean = false) {
    const config = vscode.workspace.getConfiguration('rx');
    const rxPath = config.get<string>('path', 'rx');
    const profile = config.get<string>('profile', '');
    const profileFlag = profile ? `--profile ${profile} ` : '';

    const terminal = vscode.window.createTerminal({
        name: `rx ${command}`,
        hideFromUser: silent,
    });

    terminal.sendText(`${rxPath} ${profileFlag}${command}`);

    if (!silent) {
        terminal.show();
    }
}

export function deactivate() {
    if (outputChannel) {
        outputChannel.dispose();
    }
}
