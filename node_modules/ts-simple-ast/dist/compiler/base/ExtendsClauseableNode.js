"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var manipulation_1 = require("../../manipulation");
var structurePrinters_1 = require("../../structurePrinters");
var typescript_1 = require("../../typescript");
var callBaseFill_1 = require("../callBaseFill");
function ExtendsClauseableNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.getExtends = function () {
            var extendsClause = this.getHeritageClauseByKind(typescript_1.SyntaxKind.ExtendsKeyword);
            return extendsClause == null ? [] : extendsClause.getTypeNodes();
        };
        class_1.prototype.addExtends = function (text) {
            return this.insertExtends(this.getExtends().length, text);
        };
        class_1.prototype.insertExtends = function (index, texts) {
            var length = texts instanceof Array ? texts.length : 0;
            if (typeof texts === "string") {
                errors.throwIfNotStringOrWhitespace(texts, "texts");
                texts = [texts];
            }
            else if (texts.length === 0) {
                return [];
            }
            var writer = this.getWriterWithQueuedChildIndentation();
            var structurePrinter = new structurePrinters_1.CommaSeparatedStructuresPrinter(new structurePrinters_1.StringStructurePrinter());
            structurePrinter.printText(writer, texts);
            var extendsTypes = this.getExtends();
            index = manipulation_1.verifyAndGetIndex(index, extendsTypes.length);
            if (extendsTypes.length > 0) {
                var extendsClause = this.getHeritageClauseByKindOrThrow(typescript_1.SyntaxKind.ExtendsKeyword);
                manipulation_1.insertIntoCommaSeparatedNodes({
                    parent: extendsClause.getFirstChildByKindOrThrow(typescript_1.SyntaxKind.SyntaxList),
                    currentNodes: extendsTypes,
                    insertIndex: index,
                    newText: writer.toString()
                });
                return manipulation_1.getNodeOrNodesToReturn(this.getExtends(), index, length);
            }
            var openBraceToken = this.getFirstChildByKindOrThrow(typescript_1.SyntaxKind.OpenBraceToken);
            var openBraceStart = openBraceToken.getStart();
            var isLastSpace = /\s/.test(this.getSourceFile().getFullText()[openBraceStart - 1]);
            var insertText = "extends " + writer.toString() + " ";
            if (!isLastSpace)
                insertText = " " + insertText;
            manipulation_1.insertIntoParentTextRange({
                parent: this,
                insertPos: openBraceStart,
                newText: insertText
            });
            return manipulation_1.getNodeOrNodesToReturn(this.getExtends(), index, length);
        };
        class_1.prototype.removeExtends = function (implementsNodeOrIndex) {
            var extendsClause = this.getHeritageClauseByKind(typescript_1.SyntaxKind.ExtendsKeyword);
            if (extendsClause == null)
                throw new errors.InvalidOperationError("Cannot remove an extends when none exist.");
            extendsClause.removeExpression(implementsNodeOrIndex);
            return this;
        };
        class_1.prototype.fill = function (structure) {
            callBaseFill_1.callBaseFill(Base.prototype, this, structure);
            if (structure.extends != null && structure.extends.length > 0)
                this.addExtends(structure.extends);
            return this;
        };
        return class_1;
    }(Base));
}
exports.ExtendsClauseableNode = ExtendsClauseableNode;
