"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var typescript_1 = require("../../typescript");
var utils_1 = require("../../utils");
var callBaseFill_1 = require("../callBaseFill");
function ExportableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.hasExportKeyword = function () {
            return this.getExportKeyword() != null;
        };
        class_1.prototype.getExportKeyword = function () {
            return this.getFirstModifierByKind(typescript_1.SyntaxKind.ExportKeyword);
        };
        class_1.prototype.getExportKeywordOrThrow = function () {
            return errors.throwIfNullOrUndefined(this.getExportKeyword(), "Expected to find an export keyword.");
        };
        class_1.prototype.hasDefaultKeyword = function () {
            return this.getDefaultKeyword() != null;
        };
        class_1.prototype.getDefaultKeyword = function () {
            return this.getFirstModifierByKind(typescript_1.SyntaxKind.DefaultKeyword);
        };
        class_1.prototype.getDefaultKeywordOrThrow = function () {
            return errors.throwIfNullOrUndefined(this.getDefaultKeyword(), "Expected to find a default keyword.");
        };
        class_1.prototype.isExported = function () {
            if (this.hasExportKeyword())
                return true;
            var thisSymbol = this.getSymbol();
            var sourceFileSymbol = this.getSourceFile().getSymbol();
            if (thisSymbol == null || sourceFileSymbol == null)
                return false;
            return sourceFileSymbol.getExports().some(function (e) { return e === thisSymbol || e.getAliasedSymbol() === thisSymbol; });
        };
        class_1.prototype.isDefaultExport = function () {
            if (this.hasDefaultKeyword())
                return true;
            if (!utils_1.TypeGuards.isSourceFile(this.getParentOrThrow()))
                return false;
            var thisSymbol = this.getSymbol();
            var defaultExportSymbol = this.getSourceFile().getDefaultExportSymbol();
            if (defaultExportSymbol == null || thisSymbol == null)
                return false;
            if (thisSymbol === defaultExportSymbol)
                return true;
            var aliasedSymbol = defaultExportSymbol.getAliasedSymbol();
            return thisSymbol === aliasedSymbol;
        };
        class_1.prototype.isNamedExport = function () {
            var parentNode = this.getParentOrThrow();
            return utils_1.TypeGuards.isSourceFile(parentNode) && this.hasExportKeyword() && !this.hasDefaultKeyword();
        };
        class_1.prototype.setIsDefaultExport = function (value) {
            if (value === this.isDefaultExport())
                return this;
            if (value && !utils_1.TypeGuards.isSourceFile(this.getParentOrThrow()))
                throw new errors.InvalidOperationError("The parent must be a source file in order to set this node as a default export.");
            // remove any existing default export
            var sourceFile = this.getSourceFile();
            var fileDefaultExportSymbol = sourceFile.getDefaultExportSymbol();
            if (fileDefaultExportSymbol != null)
                sourceFile.removeDefaultExport(fileDefaultExportSymbol);
            if (!value)
                return this;
            // set this node as the one to default export
            if (utils_1.TypeGuards.isAmbientableNode(this) && utils_1.TypeGuards.hasName(this) && this.isAmbient()) {
                var parentSyntaxList = this.getFirstAncestorByKindOrThrow(typescript_1.SyntaxKind.SyntaxList);
                parentSyntaxList.insertChildText(this.getChildIndex() + 1, "export default " + this.getName() + ";");
            }
            else {
                this.addModifier("export");
                this.addModifier("default");
            }
            return this;
        };
        class_1.prototype.setIsExported = function (value) {
            // remove the default keyword if it exists
            if (utils_1.TypeGuards.isSourceFile(this.getParentOrThrow()))
                this.toggleModifier("default", false);
            this.toggleModifier("export", value);
            return this;
        };
        class_1.prototype.fill = function (structure) {
            callBaseFill_1.callBaseFill(Base.prototype, this, structure);
            if (structure.isExported != null)
                this.setIsExported(structure.isExported);
            if (structure.isDefaultExport != null)
                this.setIsDefaultExport(structure.isDefaultExport);
            return this;
        };
        return class_1;
    }(Base));
}
exports.ExportableNode = ExportableNode;
