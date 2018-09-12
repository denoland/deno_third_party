"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
/* barrel:ignore */
var typescript_1 = require("./typescript");
/* tslint:disable:align */
function matchFiles(path, extensions, excludes, includes, useCaseSensitiveFileNames, currentDirectory, depth, getEntries) {
    return typescript_1.ts.matchFiles.apply(this, arguments);
}
exports.matchFiles = matchFiles;
function getFileMatcherPatterns(path, excludes, includes, useCaseSensitiveFileNames, currentDirectory) {
    return typescript_1.ts.getFileMatcherPatterns.apply(this, arguments);
}
exports.getFileMatcherPatterns = getFileMatcherPatterns;
function getEmitModuleResolutionKind(compilerOptions) {
    return typescript_1.ts.getEmitModuleResolutionKind.apply(this, arguments);
}
exports.getEmitModuleResolutionKind = getEmitModuleResolutionKind;
/* tslint:enable:align */
