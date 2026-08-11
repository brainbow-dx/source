// deno-lint-ignore-file
import * as vscode from "vscode";

import { $ } from "@ethos/dev/shell";

export class TracingSubscriber {
  constructor(
    public readonly channel: vscode.OutputChannel
  ) {
    //..
  }

  private writeLine(level: string, message: string, ...args: any[]) {
    const entryTime = new Date().toLocaleTimeString();

    if (args.length > 0) {
      message = message + ' ' + args.join(' ');
    }

    this.channel.appendLine(`[${level}] ${entryTime} - ${message}`);
  }

  public error(message: string, ...args: any[]) {
    this.writeLine("EROR", message, ...args);
  }

  public warn(message: string, ...args: any[]) {
    this.writeLine("WARN", message, ...args);
  }

  public info(message: string, ...args: any[]) {
    this.writeLine("INFO", message, ...args);
  }

  public debug(message: string, ...args: any[]) {
    this.writeLine("DEBG", message, ...args);
  }

  public trace(message: string, ...args: any[]) {
    this.writeLine("TRAC", message, ...args);
  }
}
