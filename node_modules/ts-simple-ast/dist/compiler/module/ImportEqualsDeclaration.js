"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var utils_1 = require("../../utils");
var base_1 = require("../base");
var statement_1 = require("../statement");
exports.ImportEqualsDeclarationBase = base_1.JSDocableNode(base_1.NamedNode(statement_1.Statement));
var ImportEqualsDeclaration = /** @class */ (function (_super) {
    tslib_1.__extends(ImportEqualsDeclaration, _super);
    function ImportEqualsDeclaration() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets the module reference of the import equals declaration.
     */
    ImportEqualsDeclaration.prototype.getModuleReference = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.moduleReference);
    };
    /**
     * Gets if the external module reference is relative.
     */
    ImportEqualsDeclaration.prototype.isExternalModuleReferenceRelative = function () {
        var moduleReference = this.getModuleReference();
        if (!utils_1.TypeGuards.isExternalModuleReference(moduleReference))
            return false;
        return moduleReference.isRelative();
    };
    ImportEqualsDeclaration.prototype.setExternalModuleReference = function (textOrSourceFile) {
        var text = typeof textOrSourceFile === "string" ? textOrSourceFile : this.sourceFile.getRelativePathAsModuleSpecifierTo(textOrSourceFile);
        var moduleReference = this.getModuleReference();
        if (utils_1.TypeGuards.isExternalModuleReference(moduleReference) && moduleReference.getExpression() != null)
            moduleReference.getExpressionOrThrow().replaceWithText(function (writer) { return writer.quote(text); });
        else
            moduleReference.replaceWithText(function (writer) { return writer.write("require(").quote(text).write(")"); });
        return this;
    };
    /**
     * Gets the source file referenced in the external module reference or throws if it doesn't exist.
     */
    ImportEqualsDeclaration.prototype.getExternalModuleReferenceSourceFileOrThrow = function () {
        return errors.throwIfNullOrUndefined(this.getExternalModuleReferenceSourceFile(), "Expected to find an external module reference's referenced source file.");
    };
    /**
     * Gets the source file referenced in the external module reference or returns undefined if it doesn't exist.
     */
    ImportEqualsDeclaration.prototype.getExternalModuleReferenceSourceFile = function () {
        var moduleReference = this.getModuleReference();
        if (!utils_1.TypeGuards.isExternalModuleReference(moduleReference))
            return undefined;
        return moduleReference.getReferencedSourceFile();
    };
    return ImportEqualsDeclaration;
}(exports.ImportEqualsDeclarationBase));
exports.ImportEqualsDeclaration = ImportEqualsDeclaration;
