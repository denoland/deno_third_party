"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var fileSystem_1 = require("../../fileSystem");
var TsConfigResolver_1 = require("./TsConfigResolver");
/**
 * Gets the compiler options from a specified tsconfig.json
 * @param filePath - File path to the tsconfig.json.
 * @param options - Options.
 */
function getCompilerOptionsFromTsConfig(filePath, options) {
    if (options === void 0) { options = {}; }
    // remember, this is a public function
    var fileSystemWrapper = new fileSystem_1.FileSystemWrapper(options.fileSystem || new fileSystem_1.DefaultFileSystemHost());
    var tsConfigResolver = new TsConfigResolver_1.TsConfigResolver(fileSystemWrapper, filePath, options.encoding || "utf-8");
    return {
        options: tsConfigResolver.getCompilerOptions(),
        errors: tsConfigResolver.getErrors()
    };
}
exports.getCompilerOptionsFromTsConfig = getCompilerOptionsFromTsConfig;
