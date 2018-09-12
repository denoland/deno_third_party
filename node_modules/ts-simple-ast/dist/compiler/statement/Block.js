"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../base");
var Statement_1 = require("./Statement");
var StatementedNode_1 = require("./StatementedNode");
exports.BlockBase = base_1.TextInsertableNode(StatementedNode_1.StatementedNode(Statement_1.Statement));
var Block = /** @class */ (function (_super) {
    tslib_1.__extends(Block, _super);
    function Block() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return Block;
}(exports.BlockBase));
exports.Block = Block;
