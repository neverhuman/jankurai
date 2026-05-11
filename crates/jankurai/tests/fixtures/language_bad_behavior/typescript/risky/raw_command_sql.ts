import { exec } from "child_process";

exec(req.body.command + " --verbose");
